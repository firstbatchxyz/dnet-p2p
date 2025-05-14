mod register;

impl crate::DnetService {
    /// The service type for `dnet` within mDNS.
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
    pub const MDNS_SERVICE_TYPE: &'static str = "_dnet._tcp.local.";
}
