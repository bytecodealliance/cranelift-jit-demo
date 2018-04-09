This is a simple demo that JIT-compiles a toy language to Cretonne.

It uses the new SimpleJIT interface in development
[here](https://github.com/sunfishcode/cretonne/tree/module). SimpleJIT takes care
of managing a symbol table, allocating memory, and performing relocations, offering
a relatively simple API.

This is inspired by Ulysse Carion's
[llvm-rust-getting-started](https://github.com/ucarion/llvm-rust-getting-started)
and Jonathan Turner's [rustyjit](https://github.com/jonathandturner/rustyjit).
