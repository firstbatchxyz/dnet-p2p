/// FFI-related `extern` functions for dnet.
#[cfg(feature = "ffi")]
mod ffi;
#[cfg(feature = "ffi")]
pub use ffi::*;

/// Service itself, e.g. a TCP socket.
mod service;
pub use service::{DnetService, ServiceProperties};
