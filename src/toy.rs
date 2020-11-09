extern crate cranelift;
extern crate cranelift_module;
extern crate cranelift_simplejit;
extern crate peg;

use std::mem;

mod frontend;
mod jit;

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
    }
"#;

/// Let's say hello, by calling into libc. The puts function is resolved by
/// dlsym to the libc function, and the string &hello_string is defined below.
const HELLO_CODE: &str = r#"
fn hello() -> (r) {
    puts(&hello_string)
}
"#;

fn main() {
    run_toy().unwrap();
}

fn run_code<I, O>(jit: &mut jit::JIT, code: &str, input: I) -> Result<O, String> {
    // Pass the string to the JIT, and it returns a raw pointer to machine code.
    let code_ptr = jit.compile(code)?;
    // Cast the raw pointer to a typed function pointer. This is unsafe, because
    // this is the critical point where you have to trust that the generated code
    // is safe to be called.
    let code_fn = unsafe { mem::transmute::<_, fn(I) -> O>(code_ptr) };
    // And now we can call it!
    Ok(code_fn(input))
}

fn run_toy() -> Result<(), String> {
    // Create the JIT instance, which manages all generated functions and data.
    let mut jit = jit::JIT::new();

    let result: isize = run_code(&mut jit, FOO_CODE, (1, 0))?;

    // And now we can call it!
    println!("the answer is: {}", result);

    // -------------------------------------------------------------------------//

    let result: isize = run_code(&mut jit, RECURSIVE_FIB_CODE, 10)?;

    // And we can now call it!
    println!("recursive_fib(10) = {}", result);

    // -------------------------------------------------------------------------//

    let result: isize = run_code(&mut jit, ITERATIVE_FIB_CODE, 10)?;

    // And we can now call it!
    println!("iterative_fib(10) = {}", result);

    // -------------------------------------------------------------------------//

    jit.create_data("hello_string", "hello world!\0".as_bytes().to_vec())?;
    let _: isize = run_code(&mut jit, HELLO_CODE, ())?;

    Ok(())
}
