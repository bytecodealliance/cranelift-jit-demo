Hello!

This is a simple demo that JIT-compiles a toy language, using Cranelift.

It uses the new SimpleJIT interface in development
[here](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift/simplejit). SimpleJIT takes care
of managing a symbol table, allocating memory, and performing relocations, offering
a relatively simple API.

This is inspired in part by Ulysse Carion's
[llvm-rust-getting-started](https://github.com/ucarion/llvm-rust-getting-started)
and Jonathan Turner's [rustyjit](https://github.com/jonathandturner/rustyjit).

A quick introduction to Cranelift: Cranelift is a compiler backend. It's
light-weight, supports `no_std` mode, doesn't use of floating-point itself,
and it makes efficient use of memory.

And Cranelift is being architected to allow flexibility in how one uses it.
Sometimes that flexibility can be a burden, which we've recently started to
address in a new set of crates, `cranelift-module`, `cranelift-simplejit`, and
`cranelift-faerie`, which put the pieces together in some easy-to-use
configurations for working with multiple functions at once. `cranelift-module`
is a common interface for working with multiple functions and data interfaces
at once. This interface can sit on top of `cranelift-simplejit`, which writes
code and data to memory where they can be executed and accessed. And, it can
sit on top of `cranelift-faerie`, which writes code and data to native .o files
which can be linked into native executables.

This post introduces Cranelift by walking through a simple JIT demo, using
the [`cranelift-simplejit`](https://crates.io/crates/cranelift-simplejit) crate.
Currently this demo works on Linux x86-64 platforms. It may also work on Mac
x86-64 platforms, though I haven't specifically tested that yet. And Cranelift
is being designed to support many other kinds of platforms in the future.

### A walkthrough

First, let's take a quick look at the toy language in use. It's a very
simple language, in which all variables have type `isize`. (Cranelift does have
full support for other integer and floating-point types, so this is just to
keep the toy language simple).

For a quick flavor, here's our
[first example](./src/toy.rs#L21)
in the toy language:

```
        fn foo(a, b) -> (c) {
            c = if a {
                if b {
                    30
                } else {
                    40
                }
            } else {
                50
            }
            c = c + 2
        }
```

The grammar for this toy language is defined [here](./src/frontend.rs#L23),
and this demo uses the [peg](https://crates.io/crates/peg) parser generator library
to generate actual parser code for it.

The output of parsing is a [custom AST type](./src/frontend.rs#L1):
```rust
pub enum Expr {
    Literal(String),
    Identifier(String),
    Assign(String, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    IfElse(Box<Expr>, Vec<Expr>, Vec<Expr>),
    WhileLoop(Box<Expr>, Vec<Expr>),
    Call(String, Vec<Expr>),
    GlobalDataAddr(String),
}
```

It's pretty minimal and straightforward. The `IfElse` can return a value, to show
how that's done in Cranelift (see below).

The
[first thing we do](./src/toy.rs#L13)
is create an instance of our `JIT`:
```rust
let mut jit = jit::JIT::new();
```

The `JIT` class is defined
[here](./src/jit.rs#L10)
and contains several fields:
 - `builder_context` - Cranelift uses this to reuse dynamic allocations between
   compiling multiple functions.
 - `ctx` - This is the main `Context` object for compiling functions.
 - `data_ctx` - Similar to `ctx`, but for "compiling" data sections.
 - `module` - The `Module` which holds information about all functions and data
   objects defined in the current `JIT`.

Before we go any further, let's talk about the underlying model here. The
`Module` class divides the world into two kinds of things: functions, and data
objects. Both functions and data objects have *names*, and can be imported into
a module, defined and only referenced locally, or defined and exported for use
in outside code. Functions are immutable, while data objects can be declared
either readonly or writable.

Both functions and data objects can contain references to other functions and
data objects. Cranelift is designed to allow the low-level parts operate on each
function and data object independently, so each function and data object maintains
its own individual namespace of imported names. The
[`Module`](https://docs.rs/cranelift-module/latest/cranelift_module/struct.Module.html)
struct takes care of maintaining a set of declarations for use across multiple
functions and data objects.

These concepts are sufficiently general that they're applicable to JITing as
well as native object files (more discussion below!), and `Module` provides an
interface which abstracts over both. It is parameterized with a
[`Backend`](https://docs.rs/cranelift-module/latest/cranelift_module/trait.Backend.html)
trait, which allows users to specify what underlying implementation they want to use.

Once we've
[initialized the JIT data structures](./src/jit.rs#L29),
we then use our `JIT` to
[compile](./src/jit.rs#L46)
some functions.

The `JIT`'s `compile` function takes a string containing a function in the
toy language. It
[parses](./src/jit.rs#L48)
the string into an AST, and then
[translates](./src/jit.rs#L52)
the AST into Cranelift IR.

Our toy language only supports one type, so we start by
[declaring that type](./src/jit.rs#L117)
for convenience.

We then start translating the function by adding
[the function parameters](./src/jit.rs#L121)
and
[return types](./src/jit.rs#L125)
to the Cranelift function signature.

Then we
[create](./src/jit.rs#L129)
a
[FunctionBuilder](https://docs.rs/cranelift-frontend/latest/cranelift_frontend/struct.FunctionBuilder.html)
which is a utility for building up the contents of a Cranelift IR function. As we'll
see below, `FunctionBuilder` includes functionality for constructing SSA form
automatically so that users don't have to worry about it.

Next, we
[start](./src/jit.rs#L132)
an initial basic block (block), which is the entry block of the function, and
the place where we'll insert some code.

 - A basic block is a sequence of IR instructions which have a single entry
   point, and no branches until the very end, so execution always starts at the
   top and proceeds straight through to the end.

Cranelift's basic blocks can have parameters. These take the place of
PHI functions in other IRs.

Here's an example of a block, showing branches (`brif` and `jump`) that are at the end of the block,
and demonstrating some block parameters.

```
block0(v0: i32, v1: i32, v2: i32, v507: i64):
    v508 = iconst.i32 0
    v509 = iconst.i64 0
    v404 = ifcmp_imm v2, 0
    v10 = iadd_imm v2, -7
    v405 = ifcmp_imm v2, 7
    brif ugt v405, block29(v10)
    jump block29(v508)
```

The `FunctionBuilder` library will take care of
inserting block parameters automatically, so frontends that don't need to use
them directly generally don't need to worry about them, though one place they
do come up is that incoming arguments to a function are represented as
block parameters to the entry block. We must tell Cranelift to add the parameters,
using
[`append_block_params_for_function_params`](https://docs.rs/cranelift-frontend/latest/cranelift_frontend/struct.FunctionBuilder.html#method.append_block_params_for_function_params)
like
[so](./src/jit.rs#L135).

The `FunctionBuilder` keeps track of a "current" block that new instructions are
to be inserted into; we next
[inform](./src/jit.rs#L141)
it of our new block, using
[`switch_to_block`](https://docs.rs/cranelift-frontend/latest/cranelift_frontend/struct.FunctionBuilder.html#method.switch_to_block),
so that we can start
inserting instructions into it.

The one major concept about blocks is that the `FunctionBuilder` wants to know when
all branches which could branch to a block have been seen, at which point it can
*seal* the block, which allows it to perform SSA construction. All blocks must be
sealed by the end of the function. We
[seal](./src/jit.rs#L144)
a block with
[`seal_block`](https://docs.rs/cranelift-frontend/latest/cranelift_frontend/struct.FunctionBuilder.html#method.seal_block).

Next, our toy language doesn't have explicit variable declarations, so we walk the
AST to discover all the variables, so that we can
[declare](./src/jit.rs#L149)
then to the `FunctionBuilder`. These variables need not be in SSA form; the
`FunctionBuilder` will take care of constructing SSA form internally.

For convenience when walking the function body, the demo here
[uses](./src/jit.rs#L154)
 a `FunctionTranslator` object, which holds the `FunctionBuilder`, the current
`Module`, as well as the symbol table for looking up variables. Now we can start
[walking the function body](./src/jit.rs#L161).

[AST translation](./src/jit.rs#L189)
utilizes the instruction-building features of `FunctionBuilder`. Let's start with
a simple example translating integer literals:

```rust
    Expr::Literal(literal) => {
        let imm: i32 = literal.parse().unwrap();
        self.builder.ins().iconst(self.int, i64::from(imm))
    }
```

The first part is just extracting the integer value from the AST. The next
line is the builder line:

 - The `.ins()` returns an "insertion object", which allows inserting an
   instruction at the end of the currently active block.
 - `iconst` is the name of the builder routine for creating
   [integer constants](https://cranelift.readthedocs.io/en/latest/langref.html#inst-iconst)
   in Cranelift. Every instruction in the IR can be created directly through such
   a function call.

Translation of
[Add nodes](./src/jit.rs#L199)
and other arithmetic operations is similarly straightforward.

Translation of
[variable references](./src/jit.rs#L275)
is mostly handled by `FunctionBuilder`'s `use_var` function:
```rust
    Expr::Identifier(name) => {
        // `use_var` is used to read the value of a variable.
        let variable = self.variables.get(&name).expect("variable not defined");
        self.builder.use_var(*variable)
    }
```
`use_var` is for reading the value of a (non-SSA) variable. (Internally,
`FunctionBuilder` constructs SSA form to satisfy all uses).

Its companion is `def_var`, which is used to write the value of a (non-SSA)
variable, which we use to implement assignment:
```rust
    Expr::Assign(name, expr) => {
        // `def_var` is used to write the value of a variable. Note that
        // variables can have multiple definitions. Cranelift will
        // convert them into SSA form for itself automatically.
        let new_value = self.translate_expr(*expr);
        let variable = self.variables.get(&name).unwrap();
        self.builder.def_var(*variable, new_value);
        new_value
    }
```

Next, let's dive into
[if-else](./src/jit.rs#L291)
expressions. In order to demonstrate explicit SSA construction, this demo gives
if-else expressions return values. The way this looks in Cranelift is that
the true and false arms of the if-else both have branches to a common merge point,
and they each pass their "return value" as a block parameter to the merge point.

Note that we seal the blocks we create once we know we'll have no more predecessors,
which is something that a typical AST makes it easy to know.

Putting it all together, here's the Cranelift IR for the function named
[foo](./src/toy.rs#L15)
in the demo program, which contains multiple ifs:

```
function u0:0(i64, i64) -> i64 system_v {
block0(v0: i64, v1: i64):
    v2 = iconst.i64 0
    brz v0, block2
    jump block1

block1:
    v4 = iconst.i64 0
    brz.i64 v1, block5
    jump block4

block4:
    v6 = iconst.i64 0
    v7 = iconst.i64 30
    jump block6(v7)

block5:
    v8 = iconst.i64 0
    v9 = iconst.i64 40
    jump block6(v9)

block6(v5: i64):
    jump block3(v5)

block2:
    v10 = iconst.i64 0
    v11 = iconst.i64 50
    jump block3(v11)

block3(v3: i64):
    v12 = iconst.i64 2
    v13 = iadd v3, v12
    return v13
}
```

The [while loop](./src/jit.rs#L338)
translation is also straightforward.

Here's the Cranelift IR for the function named [iterative_fib](./src/toy.rs#L81)
in the demo program, which contains a while loop:

```
function u0:0(i64) -> i64 system_v {
block0(v0: i64):
    v1 = iconst.i64 0
    v2 = iconst.i64 0
    v3 = icmp eq v0, v2
    v4 = bint.i64 v3
    brz v4, block2
    jump block1

block1:
    v6 = iconst.i64 0
    v7 = iconst.i64 0
    jump block3(v7, v7)

block2:
    v8 = iconst.i64 0
    v9 = iconst.i64 1
    v10 = isub.i64 v0, v9
    v11 = iconst.i64 0
    v12 = iconst.i64 1
    jump block4(v10, v12, v11)

block4(v13: i64, v17: i64, v18: i64):
    v14 = iconst.i64 0
    v15 = icmp ne v13, v14
    v16 = bint.i64 v15
    brz v16, block6
    jump block5

block5:
    v19 = iadd.i64 v17, v18
    v20 = iconst.i64 1
    v21 = isub.i64 v13, v20
    jump block4(v21, v19, v17)

block6:
    v22 = iconst.i64 0
    jump block3(v22, v17)

block3(v5: i64, v23: i64):
    return v23
}
```

For
[calls](./src/jit.rs#L365),
the basic steps are to determine the call signature, declare the function to be
called, put the values to be passed in an array, and then call the `call` function.

The translation for
[global data symbols](./src/jit.rs#L393),
is similar; first declare the symbol to the module, then declare it to the current
function, and then use the `symbol_value` instruction to produce the value.

And with that, we can return to our main `toy.rs` file and run some more examples.
There are examples of recursive and iterative fibonacci, which demonstrate more use
of calls and control flow.

And there's a hello world example which demonstrates several other features.

This program needs to allocate some
[data](./src/toy.rs#L120)
to hold the string data. Inside jit.rs,
[`create_data`](./src/jit.rs#L90)
we initialize a `DataContext` with the contents of the hello string, and also
declare a data object. Then we use the `DataContext` object to define the object.
At that point, we're done with the `DataContext` object and can clear it. We
then call `finalize_data` to perform linking (although our simple hello string
doesn't make any references so there isn't anything to do) and to obtain the
final runtime address of the data, which we then convert back into a Rust slice
for convenience.

And to show off a handy feature of the simplejit backend, it can look up symbols
with `libc::dlsym`, so you can call libc functions such as `puts` (being careful
to NUL-terminate your strings!). Unfortunately, `printf` requires varargs, which
Cranelift does not yet support.

And with all that, we can say "hello world!".


### Native object files

Because of the `Module` abstraction, this demo can be adapted to write out an ELF
.o file rather than JITing the code to memory with only minor changes, and I've done
so in a branch [here](https://github.com/bytecodealliance/simplejit-demo/tree/faerie).
This writes a `test.o` file, which on an x86-64 ELF platform you can link with
`cc test.o` and it produces an executable that calls the generated functions,
including printing "hello world!".

Another branch [here](https://github.com/bytecodealliance/simplejit-demo/tree/faerie-macho)
shows how to write Mach-O object files.

Object files are written using the
[faerie](https://github.com/m4b/faerie) library.

### Have fun!

Cranelift is still evolving, so if there are things here which are confusing or
awkward, please let us know, via
[github issues](https://github.com/bytecodealliance/cranelift/issues) or
just stop by the [gitter chat](https://gitter.im/CraneStation/Lobby/~chat).
Very few things in Cranelift's design are set in stone at this time, and we're
really interested to hear from people about what makes sense what doesn't.
