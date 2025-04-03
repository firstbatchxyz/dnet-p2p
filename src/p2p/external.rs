//! FFI-supporting `extern` functions for the `p2p` module.
//!
//! This module contains functions that are callable from C/C++ code.
//! To be more type-safe, one can create a dummy struct in C/C++ and use it as a pointer.
//!
//! ```c
//! typedef struct dllmd dllmd_t;
//! ```
//!
//! Each function in this module is prefixed with `dllmd_` to avoid name clashes.
//! They also have their declarations within their docstrings.
//!
//! Inspired from: https://jakegoulding.com/rust-ffi-omnibus/objects/

use std::ffi::c_char;

use super::DLLMP2P;

/// Creates a new `DLLMP2P` instance.
///
/// To be declared in C/C++ as:
/// ```c
/// extern dllmd_t* dllmd_new(void);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_new() -> *mut DLLMP2P {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let dllmp2p = DLLMP2P::new(keypair).expect("could not create DLLMP2P");
    Box::into_raw(Box::new(dllmp2p))
}

pub extern "C" fn dllmd_listen_on(ptr: *mut DLLMP2P, addr: *const c_char) {
    // allow null pointer to be passed
    if ptr.is_null() {
        return;
    }

    let addr = unsafe { std::ffi::CStr::from_ptr(addr) };
    let addr = addr.to_str().unwrap();
    let addr = addr.parse().expect("could not parse address");

    // since the object was allocated by Rust, it must be freed by Rust as well;
    // so we use `Box::from_raw` to convert the raw pointer back into a `Box` and then drop it.
    unsafe {
        (*ptr).listen_on(Some(addr));
    }
}

/// Frees the memory allocated for the `DLLMP2P` instance.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void dllmd_free(dllmd_t* ptr);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_free(ptr: *mut DLLMP2P) {
    // allow null pointer to be passed
    if ptr.is_null() {
        return;
    }

    // since the object was allocated by Rust, it must be freed by Rust as well;
    // so we use `Box::from_raw` to convert the raw pointer back into a `Box` and then drop it.
    unsafe {
        drop(Box::from_raw(ptr));
    }
}
