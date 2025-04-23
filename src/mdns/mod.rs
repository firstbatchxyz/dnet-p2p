mod browse;
pub use browse::browse_mdns;

mod register;
pub use register::register_mdns;

pub const DNET_SERVICE_TYPE: &str = "_dnet._tcp.local.";
