use core::mem;
use cranelift_jit_demo::jit;

fn main() -> Result<(), String> {
    // Create the JIT instance, which manages all generated functions and data.
    let mut jit = jit::JIT::default();
    println!("the answer is: {}", run_foo(&mut jit)?);
    println!(
        "recursive_fib(10) = {}",
        run_recursive_fib_code(&mut jit, 10)?
    );
    println!(
        "iterative_fib(10) = {}",
        run_iterative_fib_code(&mut jit, 10)?
    );
    run_call_builtin_symbol(&mut jit)?;
    run_hello(&mut jit)?;
    Ok(())
}

fn run_foo(jit: &mut jit::JIT) -> Result<isize, String> {
    unsafe { run_code(jit, FOO_CODE, (1, 0)) }
}

fn run_recursive_fib_code(jit: &mut jit::JIT, input: isize) -> Result<isize, String> {
    unsafe { run_code(jit, RECURSIVE_FIB_CODE, input) }
}

fn run_iterative_fib_code(jit: &mut jit::JIT, input: isize) -> Result<isize, String> {
    unsafe { run_code(jit, ITERATIVE_FIB_CODE, input) }
}

fn run_call_builtin_symbol(jit: &mut jit::JIT) -> Result<isize, String> {
    unsafe {
        jit.create_data("example_string", *b"Example String\0")?;
        run_code(jit, CALL_BUILTIN_SYMBOL_CODE, ())
    }
}
fn run_hello(jit: &mut jit::JIT) -> Result<isize, String> {
    unsafe {
        jit.create_data("hello_string", *b"hello world!\0")?;
        run_code(jit, HELLO_CODE, ())
    }
}


/// Executes the given code using the cranelift JIT compiler.
///
/// Feeds the given input into the JIT compiled function and returns the resulting output.
///
/// # Safety
///
/// This function is unsafe since it relies on the caller to provide it with the correct
/// input and output types. Using incorrect types at this point may corrupt the program's state.
unsafe fn run_code<I, O>(jit: &mut jit::JIT, code: &str, input: I) -> Result<O, String> {
    // Pass the string to the JIT, and it returns a raw pointer to machine code.
    let code_ptr = jit.compile(code)?;
    // Cast the raw pointer to a typed function pointer. This is unsafe, because
    // this is the critical point where you have to trust that the generated code
    // is safe to be called.
    let code_fn = mem::transmute::<_, fn(I) -> O>(code_ptr);
    // And now we can call it!
    Ok(code_fn(input))
}

// A small test function.
//
// The `(c)` declares a return variable; the function returns whatever value
// it was assigned when the function exits. Note that there are multiple
// assignments, so the input is not in SSA form, but that's ok because
// Cranelift handles all the details of translating into SSA form itself.
const FOO_CODE: &str = r#"
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
        assert_int(c, 42)
    }
"#;

/// Another example: Recursive fibonacci.
const RECURSIVE_FIB_CODE: &str = r#"
    fn recursive_fib(n) -> (r) {
        r = if n == 0 {
                    0
            } else {
                if n == 1 {
                    1
                } else {
                    recursive_fib(n - 1) + recursive_fib(n - 2)
                }
            }
    }
"#;

/// Another example: Iterative fibonacci.
const ITERATIVE_FIB_CODE: &str = r#"
    fn iterative_fib(n) -> (r) {
        if n == 0 {
            r = 0
        } else {
            n = n - 1
            a = 0
            r = 1
            while n != 0 {
                t = r
                r = r + a
                a = t
                n = n - 1
            }
        }
        assert_int(r, 55)
    }
"#;

const CALL_BUILTIN_SYMBOL_CODE: &str = r#"
fn call_builtin_symbol() -> (r) {
    println_string(&example_string)
    println_int(10)
}
"#;

/// Let's say hello, by calling into libc. The puts function is resolved by
/// dlsym to the libc function, and the string &hello_string is defined below.
const HELLO_CODE: &str = r#"
fn hello() -> (r) {
    puts(&hello_string)
}
"#;
