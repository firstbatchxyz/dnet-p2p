use mdns_sd::ServiceInfo;

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

    /// Registers a service with the given instance name and hostname.
    ///
    /// - `instance_name`: The name of the service instance, e.g. `worker-1`
    /// - `hostname`: The hostname of the service, e.g. `john-doe-macbook`
    /// - `service_port`: The port that the [`crate::DnetService`] is listening on.
    ///
    /// FIXME: if the same `instance_name` exists, it will be renamed (e.g. `foo` becomes `foo (2)`, `foo (3)` and so on)
    /// so we need to know that and unregister with the correct name.
    pub async fn register(&self) -> eyre::Result<String> {
        // register your own hostname
        log::debug!(
            "Registering {} of host {}",
            self.instance_name,
            self.hostname
        );
        let service_info = ServiceInfo::new(
            Self::MDNS_SERVICE_TYPE,
            &self.instance_name,
            &format!("{}.local.", self.hostname),
            "", // thanks to `enable_addr_auto` we can give this as empty string
            self.port,
            &self.properties,
        )
        .expect("valid service info")
        // automatically update the addresses of this service, when IP address(es) are added or removed on the host
        .enable_addr_auto();

        let service_fullname = service_info.get_fullname().to_string();
        self.mdns
            .register(service_info)
            .expect("Failed to register mDNS service");

        log::info!("Registered service {service_fullname}",);

        Ok(service_fullname)
    }
}
