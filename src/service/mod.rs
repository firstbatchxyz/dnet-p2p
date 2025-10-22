/// Core service functionality.
mod core;
pub use core::DnetService;

/// Service properties, also used in mDNS as `TXT` records.
mod properties;
pub use properties::DnetServiceProperties;

/// UDP discovery module.
pub(crate) mod udp;

// /// mDNS specific functions.
// mod mdns;

/// Thunderbolt-specific info.
mod thunderbolt;
pub use thunderbolt::ThunderboltData;
