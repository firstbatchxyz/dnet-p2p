/// FFI-related `extern` functions for dnet.
mod ffi;
pub use ffi::*;

/// Service itself, e.g. a TCP socket.
mod service;

/// Wrapper of [`mdns-sd`]
mod mdns;
pub use mdns::{browse_mdns, register_mdns};
