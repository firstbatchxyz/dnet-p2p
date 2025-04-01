use std::ffi::{c_char, CStr, CString};

/// Given a string input, returns the same.
///
/// Should be used for testing purposes of the FFI logic.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn echo(input: *const c_char) -> CString {
    let input_cstr = unsafe {
        assert!(!input.is_null());
        CStr::from_ptr(input)
    };
    let input = input_cstr.to_str().unwrap().to_string();
    CString::new(input).unwrap()
}
