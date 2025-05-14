mod browse;
mod register;

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::service::ServiceProperties;

pub struct DnetMDNSDameon {
    /// The cancellation token to cancel the daemon.
    cancellation: CancellationToken,
    /// Discovered services.
    /// TODO: currently full-name and address
    peers: HashMap<String, String>,
    /// A shared service properties object.
    properties: Arc<RwLock<ServiceProperties>>,
}

impl crate::DnetService {
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
}
