extern crate cretonne;
extern crate cton_frontend;
extern crate cton_module;
extern crate cton_simplejit;

use std::process;
use std::mem;

mod jit;

fn main() {
    // Create the JIT instance, which manages all generated functions and data.
    let mut jit = jit::JIT::new();

    // A small test function.
    //
    // The `(c)` declares a return variable; the function returns whatever value
    // it was assigned when the function exits. Note that there are multiple
    // assignments, so the input is not in SSA form, but that's ok because
    // Cretonne handles all the details of translating into SSA form itself.
    let foo_code = "\
        fn foo(a, b) -> (c) {      \n\
            c = if a {             \n\
                if b {             \n\
                    30             \n\
                } else {           \n\
                    40             \n\
                }                  \n\
            } else {               \n\
                50                 \n\
            }                      \n\
            c = c + 2              \n\
        }                          \n\
    ";

    // Pass the string to the JIT, and it returns a raw pointer to machine code.
    let foo = jit.compile(&foo_code).unwrap_or_else(|msg| {
        eprintln!("error: {}", msg);
        process::exit(1);
    });

    // Cast the raw pointer to a typed function pointer. This is unsafe, because
    // this is the critical point where you have to trust that the generated code
    // is safe to be called.
    //
    // TODO: Is there a way to fold this transmute into `compile` above?
    let foo = unsafe { mem::transmute::<_, fn(i32, i32) -> i32>(foo) };

    // And now we can call it!
    println!("the answer is: {}", foo(1, 0));


    // -------------------------------------------------------------------------//

    // Another example: Recursive fibonacci.
    let recursive_fib_code = "\
        fn recursive_fib(n) -> (r) {                                      \n\
            r = if n == 0 {                                               \n\
                     0                                                    \n\
                } else {                                                  \n\
                    if n == 1 {                                           \n\
                        1                                                 \n\
                    } else {                                              \n\
                        recursive_fib(n - 1) + recursive_fib(n - 2)       \n\
                    }                                                     \n\
                }                                                         \n\
        }                                                                 \n\
    ";

    // Same as above.
    let recursive_fib = jit.compile(&recursive_fib_code).unwrap_or_else(|msg| {
        eprintln!("error: {}", msg);
        process::exit(1);
    });
    let recursive_fib = unsafe { mem::transmute::<_, fn(i32) -> i32>(recursive_fib) };

    // And we can now call it!
    println!("recursive_fib(10) = {}", recursive_fib(10));


    // -------------------------------------------------------------------------//

    // Another example: Iterative fibonacci.
    let iterative_fib_code = "\
        fn iterative_fib(n) -> (r) {                                      \n\
            if n == 0 {                                                   \n\
                r = 0                                                     \n\
            } else {                                                      \n\
                n = n - 1                                                 \n\
                a = 0                                                     \n\
                r = 1                                                     \n\
                while n != 0 {                                            \n\
                    t = r                                                 \n\
                    r = r + a                                             \n\
                    a = t                                                 \n\
                    n = n - 1                                             \n\
                }                                                         \n\
            }                                                             \n\
        }                                                                 \n\
    ";

    // Same as above.
    let iterative_fib = jit.compile(&iterative_fib_code).unwrap_or_else(|msg| {
        eprintln!("error: {}", msg);
        process::exit(1);
    });
    let iterative_fib = unsafe { mem::transmute::<_, fn(i32) -> i32>(iterative_fib) };

    // And we can now call it!
    println!("iterative_fib(10) = {}", iterative_fib(10));
}
