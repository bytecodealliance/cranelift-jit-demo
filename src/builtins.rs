use std::ffi::{c_char, CStr};

pub unsafe extern fn println_string(string: *const c_char) {
    let cstr = CStr::from_ptr(string);
    println!("{}", cstr.to_str().unwrap())
}

pub extern fn println_int(int: isize) {
    println!("{}", int)
}

pub extern fn assert_int(int: isize, pre: isize) {
    assert_eq!(int, pre)
}
