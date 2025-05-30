//! `extern` functions for FFI-support.
//!
//! This module contains functions that are callable from C/C++ code.
//! To be more type-safe, one can create a dummy struct in C/C++ and use it as a pointer.
//!
//! ```c
//! typedef struct dnet_p2p dnet_p2p_t;
//! typedef struct dnet_p2p_handle dnet_p2p_handle_t;
//! ```
//!
//! Each function in this module is prefixed with `dnet_p2p_` to avoid name clashes.
//! They also have their declarations within their docstrings.

#![allow(clippy::missing_safety_doc)]

use std::{ffi, thread::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::DnetService;

/// Type that is returned by [`dnet_p2p_start`].
type DnetServiceHandle = JoinHandle<eyre::Result<()>>;

/// Enables logging for `dnet_p2p` while respecting
/// the `RUST_LOG` environment variable.
///
/// ---
/// C/C++ declaration:
/// ```c
/// extern void dnet_p2p_enable_logs(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn dnet_p2p_enable_logs() {
    if let Err(err) = env_logger::builder()
        .filter(None, log::LevelFilter::Off)
        .filter_module("dnet_p2p", log::LevelFilter::Info)
        .parse_default_env() // reads RUST_LOG environment variable
        .try_init()
    {
        // only possible if the logger was already initialized
        eprintln!("Could not enable logs: {err}");
    }
}

/// Creates a new `dnet_p2p` service.
///
/// Must be freed with [`dnet_p2p_free`], otherwise will cause a **memory leak**.
///
/// ---
/// C/C++ declaration:
/// ```c
/// extern dnet_p2p_t* dnet_p2p_new(void);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn dnet_p2p_new(
    instance_name_c: *const ffi::c_char,
    hostname_c: *const ffi::c_char,
    is_manager: bool,
) -> *mut DnetService {
    let [instance_name, hostname] = [instance_name_c, hostname_c].map(|ptr| {
        unsafe {
            assert!(!ptr.is_null());
            ffi::CStr::from_ptr(ptr)
        }
        .to_str()
        .unwrap()
    });

    let service = DnetService::new(CancellationToken::new(), instance_name.to_string(), hostname.to_string(), is_manager).unwrap(/* TODO: !!! */);
    Box::into_raw(Box::new(service))
}

/// Frees the memory allocated by [`dnet_p2p_new`].
///
/// Does no action if the pointer is `NULL`.
///
///
/// ---
/// C/C++ declaration:
/// ```c
/// extern void dnet_p2p_free(dnet_p2p_t* service_ptr);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dnet_p2p_free(service_ptr: *mut DnetService) {
    if service_ptr.is_null() {
        return;
    }

    // since the object was allocated by Rust, it must be freed by Rust as well;
    // so we use `Box::from_raw` to convert the raw pointer back into a `Box` and then drop it (explicitly).
    unsafe {
        drop(Box::from_raw(service_ptr));
    }
}

/// Starts the client in a new thread, and returns a join handle.
///
/// The returned handle should be passed to [`dnet_p2p_stop`] to stop the daemon gracefully.
///
///
/// ---
/// C/C++ declaration:
/// ```c
/// extern dnet_p2p_handle_t* dnet_p2p_start(dnet_p2p_t* service_ptr);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dnet_p2p_start(service_ptr: *mut DnetService) -> *mut DnetServiceHandle {
    let service = unsafe {
        assert!(!service_ptr.is_null());
        &mut *service_ptr
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("could not create runtime");

    let handle = std::thread::spawn(move || rt.block_on(async { service.start().await }));
    Box::into_raw(Box::new(handle))
}

/// Gracefully shutdown the service.
///
/// This first calls [`DnetService::stop`] on the given `service_ptr`, and then waits for the handle to finish.
/// It is expected to finish due to the internal cancellation token.
///
///
/// ---
/// C/C++ declaration:
/// ```c
/// extern int dnet_p2p_stop(dnet_p2p_t* service_ptr, dnet_p2p_handle_t* handle_ptr);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dnet_p2p_stop(
    service_ptr: *mut DnetService,
    handle_ptr: *mut JoinHandle<()>,
) -> i32 {
    let service = unsafe {
        assert!(!service_ptr.is_null(), "service_ptr is null");
        &mut *service_ptr
    };
    let handle = unsafe {
        assert!(!handle_ptr.is_null(), "handle_ptr is null");
        Box::from_raw(handle_ptr)
    };

    // stop the service gracefully by triggering the cancellation token
    service.trigger_cancellation();

    // handle should be terminated by now as we stopped the service
    match handle.join() {
        Ok(_) => 0,
        Err(err) => {
            log::error!("Could not stop the client: {err:?}");
            -1
        }
    }
}
