//! This module contains several echo functions for testing purposes.

use std::ffi::{c_char, CStr, CString};

/// Given a string input, returns it as uppercased.
///
/// To be declared in C/C++ as:
/// ```c
/// extern char* echo_upper(const char* input);
/// ```
#[no_mangle]
#[allow(improper_ctypes_definitions)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn echo_upper(input: *const c_char) -> CString {
    let input_cstr = unsafe {
        assert!(!input.is_null());
        CStr::from_ptr(input)
    };
    let input = input_cstr.to_str().unwrap().to_uppercase().to_string();
    CString::new(input).unwrap()
}

/// Given a string input, writes its uppercase version to the output pointer.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void echo_upper_mut(const char* input, char* output);
/// ```
#[no_mangle]
#[allow(improper_ctypes_definitions)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn echo_upper_mut(input: *const c_char, output: *mut c_char) {
    let input_cstr = unsafe {
        assert!(!input.is_null());
        CStr::from_ptr(input)
    };
    let upper = input_cstr.to_str().unwrap().to_uppercase();
    let output_cstring = CString::new(upper).unwrap();

    unsafe {
        assert!(!output.is_null());
        std::ptr::copy(
            output_cstring.as_ptr(),
            output,
            output_cstring.as_bytes().len() + 1,
        );
    }
}
