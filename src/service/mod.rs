/// Core service functionality.
mod core;
pub use core::DnetService;

/// Service properties, also used in mDNS as `TXT` records.
mod properties;
pub use properties::ServiceProperties;

/// mDNS specific functions.
mod mdns;
