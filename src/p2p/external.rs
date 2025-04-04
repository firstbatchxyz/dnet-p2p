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

use debug_print::debug_println;
use std::ffi::c_char;

use super::DllmP2p;

/// Creates a new `DLLMP2P` instance.
///
/// To be declared in C/C++ as:
/// ```c
/// extern dllmd_t* dllmd_new(void);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_new() -> *mut DllmP2p {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    debug_println!("Creating DLLMP2P with keypair: {:?}", keypair.public());
    let dllmp2p = DllmP2p::new(keypair).expect("could not create DLLMP2P");
    Box::into_raw(Box::new(dllmp2p))
}

/// Gracefully shutsdown the daemon.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void dllmd_shutdown(dllmd_t* ptr);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_shutdown(ptr: *mut DllmP2p) {
    let dllm = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("could not create runtime")
        .block_on(async {
            dllm.shutdown();
        });
}

/// Frees the memory allocated for the `DLLMP2P` instance.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void dllmd_free(dllmd_t* ptr);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_free(ptr: *mut DllmP2p) {
    debug_println!("Freeing");

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

/// Starts the daemon in a background thread.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void dllmd_start_daemon(dllmd_t* ptr, const char* addr);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_start_daemon(ptr: *mut DllmP2p, addr_ptr: *const c_char) {
    let addr: Option<libp2p::Multiaddr> = if addr_ptr.is_null() {
        None
    } else {
        let addr = unsafe { std::ffi::CStr::from_ptr(addr_ptr) };
        Some(
            addr.to_str()
                .expect("could not read address")
                .parse()
                .expect("could not parse address"),
        )
    };

    let dllm = unsafe {
        assert!(!ptr.is_null());
        &mut *ptr
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("could not create runtime");

    // FIXME: handle is dropped here
    std::thread::spawn(move || {
        debug_println!("Starting the node!");
        let cancel_token = tokio_util::sync::CancellationToken::new();
        rt.block_on(dllm.run_daemon(cancel_token, addr));
    });
}

/// Checks if the daemon is running.
///
/// To be declared in C/C++ as:
/// ```c
/// extern bool dllmd_is_running(dllmd_t* ptr);
/// ```
#[no_mangle]
pub extern "C" fn dllmd_is_running(ptr: *mut DllmP2p) -> bool {
    let dllm = unsafe {
        assert!(!ptr.is_null());
        &*ptr
    };

    dllm.is_running()
}
