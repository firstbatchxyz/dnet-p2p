mod browse;
mod register;

use std::collections::HashMap;

use tokio_util::sync::CancellationToken;

use crate::service::Properties;

pub struct DnetMDNSDameon {
    /// The cancellation token to cancel the daemon.
    cancellation: CancellationToken,
    /// The properties of the service that we are running.
    properties: Properties,
    /// Discovered services.
    /// TODO: currently full-name and address
    peers: HashMap<String, String>,
}

impl DnetMDNSDameon {
    /// The service type for dnet.
    ///
    /// Can be used to browse for `dnet` services, e.g. in MacOS:
    ///
    /// ```sh
    /// dns-sd -B _dnet._tcp.
    /// ```
    ///
    /// or alternatively:
    ///
    /// ```sh
    /// dns-sd -Q _dnet._tcp.local. PTR
    /// ```
    ///
    /// Note that a service type always ends with either `._tcp.local.` or `._udp.local.` in mDNS.
    pub const SERVICE_TYPE: &'static str = "_dnet._tcp.local.";

    /// Creates a new daemon that can be cancelled with the given cancellation token.
    pub fn new(cancellation: CancellationToken) -> Self {
        Self {
            cancellation,
            properties: Properties::new(),
            peers: HashMap::new(),
        }
    }
}
