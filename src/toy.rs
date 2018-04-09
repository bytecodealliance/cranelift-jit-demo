extern crate cretonne;
extern crate cton_frontend;
extern crate cton_module;
extern crate cton_simplejit;

use std::process;
use std::mem;

mod jit;

fn main() {
    let mut jit = jit::JIT::new();

    let foo_code = "\
        fn foo(a, b) -> (d) {      \n\
            c = if a {             \n\
                if b {             \n\
                    30             \n\
                } else {           \n\
                    40             \n\
                }                  \n\
            } else {               \n\
                50                 \n\
            }                      \n\
            d = c + 2              \n\
        }                          \n\
    ";

    let foo = jit.compile(&foo_code).unwrap_or_else(|msg| {
        eprintln!("error: {}", msg);
        process::exit(1);
    });

    // TODO: Is there a way to fold this transmute into `compile` above?
    let foo = unsafe { mem::transmute::<_, fn(i32, i32) -> i32>(foo) };

    println!("the answer is: {}", foo(1, 0));


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

    let recursive_fib = jit.compile(&recursive_fib_code).unwrap_or_else(|msg| {
        eprintln!("error: {}", msg);
        process::exit(1);
    });

    let recursive_fib = unsafe { mem::transmute::<_, fn(i32) -> i32>(recursive_fib) };

    println!("recursive_fib(10) = {}", recursive_fib(10));
}
