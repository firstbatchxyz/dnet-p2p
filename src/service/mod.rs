/// Core service functionality.
mod core;
pub use core::DnetService;

/// Service properties.
mod properties;
pub use properties::DnetServiceProperties;

/// UDP discovery module.
pub(crate) mod udp;

/// Thunderbolt-specific info.
mod thunderbolt;
pub use thunderbolt::ThunderboltData;
