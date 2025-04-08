//! FFI-supporting `extern` functions for the `p2p` module.
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
//!
//! Inspired from: https://jakegoulding.com/rust-ffi-omnibus/objects/

use debug_print::debug_eprintln;
use libp2p::gossipsub::PublishError;
use std::{ffi::c_char, thread::JoinHandle, time::Duration};

use super::DllmP2p;

/// Creates a new `DLLMP2P` instance.
///
/// To be declared in C/C++ as:
/// ```c
/// extern dnet_t* dnet_new(void);
/// ```
#[no_mangle]
pub extern "C" fn dnet_new() -> *mut DllmP2p {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    debug_eprintln!("Creating DLLMP2P with keypair: {:?}", keypair.public());
    let cancellation = tokio_util::sync::CancellationToken::new();
    let dllmp2p = DllmP2p::new(keypair, cancellation).expect("could not create DLLMP2P");
    Box::into_raw(Box::new(dllmp2p))
}

/// Gracefully shutsdown the daemon.
///
/// This first calls `stop()` on the `DLLMP2P` instance, and then waits for the handle to finish.
/// It is expected to finish due to the internal cancellation token.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void dnet_stop(dnet_t* ptr, dnet_handle_t* handle_ptr);
/// ```
#[no_mangle]
pub extern "C" fn dnet_stop(dllm_ptr: *mut DllmP2p, handle_ptr: *mut JoinHandle<()>) {
    let dllm = unsafe {
        assert!(!dllm_ptr.is_null(), "dllm_ptr is null");
        &mut *dllm_ptr
    };
    // if stop() is called, we need to wait for the handle to finish
    let handle = unsafe {
        assert!(!handle_ptr.is_null(), "handle_ptr is null");
        Box::from_raw(handle_ptr)
    };

    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("could not create runtime")
        .block_on(async {
            dllm.stop();
        });

    handle.join().expect("could not join handle");
}

/// Frees the memory allocated for the `DLLMP2P` instance.
///
/// To be declared in C/C++ as:
/// ```c
/// extern void dnet_free(dnet_t* ptr);
/// ```
///
/// Does no action if the pointer is `NULL`.
#[no_mangle]
pub extern "C" fn dnet_free(dllm_ptr: *mut DllmP2p) {
    if dllm_ptr.is_null() {
        return;
    }

    // since the object was allocated by Rust, it must be freed by Rust as well;
    // so we use `Box::from_raw` to convert the raw pointer back into a `Box` and then drop it.
    unsafe {
        drop(Box::from_raw(dllm_ptr));
    }
}

/// Starts the daemon in a background thread.
///
/// To be declared in C/C++ as:
/// ```c
/// extern dnet_handle_t* dnet_start(dnet_t* ptr, const char* addr);
/// ```
///
/// The returned handle should be passed to [`dnet_stop()`] to stop the daemon gracefully.
#[no_mangle]
pub extern "C" fn dnet_start(
    dllm_ptr: *mut DllmP2p,
    addr_ptr: *const c_char,
) -> *mut JoinHandle<()> {
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
        assert!(!dllm_ptr.is_null());
        &mut *dllm_ptr
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("could not create runtime");

    let handle = std::thread::spawn(move || {
        debug_eprintln!("Starting the node!");
        rt.block_on(dllm.run_daemon(addr));
    });

    Box::into_raw(Box::new(handle))
}

/// Sends raw bytes to all peers in the network.
///
/// To be declared in C/C++ as:
/// ```c
/// extern int dnet_publish(dnet_t* ptr, const char* data, size_t data_len);
/// ```
///
/// Returns non-zero on error.
#[no_mangle]
pub fn dnet_publish(dllm_ptr: *mut DllmP2p, data_ptr: *const u8, data_len: usize) -> i32 {
    const DNET_PUBLISH_ERR_INSUFFICIENT_PEERS: i32 = -1;
    const DNET_PUBLISH_ERR_MSG_TOO_LARGE: i32 = -2;
    const DNET_PUBLISH_ERR_UNHANDLED: i32 = -3;

    let dllm = unsafe {
        assert!(!dllm_ptr.is_null());
        &mut *dllm_ptr
    };

    let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };

    match dllm.publish(DllmP2p::DLLM_TOPIC, data) {
        Ok(_) => 0,
        Err(publish_err) => match publish_err {
            PublishError::MessageTooLarge => DNET_PUBLISH_ERR_MSG_TOO_LARGE,
            PublishError::InsufficientPeers => DNET_PUBLISH_ERR_INSUFFICIENT_PEERS,
            _ => {
                debug_eprintln!("Unhandled error: {:?}", publish_err);
                DNET_PUBLISH_ERR_UNHANDLED
            }
        },
    }
}

/// Waits for a message to be received from the network.
///
/// To be declared in C/C++ as:
/// ```c
/// extern int dnet_receive(dnet_t *ptr, void *buf, size_t buf_size, uint64_t timeout_ms);
/// ```
///
/// Returns the number of bytes received on success; othewrwise, returns -1.
#[no_mangle]
pub fn dnet_receive(
    dllm_ptr: *mut DllmP2p,
    buf: *const u8,
    buf_size: usize,
    timeout_ms: u64,
) -> i32 {
    let dllm = unsafe {
        assert!(!dllm_ptr.is_null());
        &mut *dllm_ptr
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("could not create runtime")
        .block_on(async {
            use tokio::time::timeout;

            match if timeout_ms != 0 {
                timeout(Duration::from_millis(timeout_ms), dllm.message_rx.recv()).await
            } else {
                Ok(dllm.message_rx.recv().await)
            } {
                Ok(Some(message)) => {
                    let data = message.data;
                    let data_len: usize = data.len();

                    if data_len == 0 {
                        // if the message is empty, we cannot copy the data
                        // but the message is consumed
                        return 0;
                    } else if buf_size < data_len {
                        // if the buffer is too small, we cannot copy the data
                        // but the message is consumed
                        // FIXME: can use `dllm.message_rx.iter().peekable();` to avoid this
                        -3
                    } else {
                        unsafe {
                            std::ptr::copy_nonoverlapping(data.as_ptr(), buf as *mut u8, data_len);
                        }
                        data_len as i32
                    }
                }
                Ok(None) => {
                    debug_eprintln!("Receive channel closed");
                    return -2;
                }
                Err(_) => {
                    return 0;
                }
            }
        })

    // blocks until a message is received
}
