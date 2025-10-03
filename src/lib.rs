/// FFI-related `extern` functions for dnet.
mod ffi;
pub use ffi::*;

/// Service itself, e.g. a TCP socket.
mod service;
pub use service::{DnetService, DnetServiceProperties};

pub mod utils;
