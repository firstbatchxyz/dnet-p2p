mod browse;
mod register;

use tokio_util::sync::CancellationToken;

pub struct DnetMDNSDameon {
    cancellation: CancellationToken,
}

impl DnetMDNSDameon {
    pub const SERVICE_TYPE: &'static str = "_dnet._tcp.local.";
    pub fn new(cancellation: CancellationToken) -> Self {
        Self { cancellation }
    }
}
