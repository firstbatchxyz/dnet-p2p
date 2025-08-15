use mdns_sd::{ServiceInfo, UnregisterStatus};
use std::time::Duration;

impl crate::DnetService {
    /// The service type for `dnet` within mDNS.
    ///
    /// Can be used to browse for `dnet_p2p` services, e.g. in MacOS:
    ///
    /// ```sh
    /// dns-sd -B _dnet_p2p._tcp.
    /// ```
    ///
    /// or alternatively:
    ///
    /// ```sh
    /// dns-sd -Q _dnet_p2p._tcp.local. PTR
    /// ```
    ///
    /// Note that a service type always ends with either `._tcp.local.` or `._udp.local.` in mDNS.
    pub const MDNS_SERVICE_TYPE: &'static str = "_dnet_p2p._tcp.local.";

    /// Registers a service with the given instance name and hostname.
    ///
    /// - `instance_name`: The name of the service instance, e.g. `worker-1`
    /// - `hostname`: The hostname of the service, e.g. `john-doe-macbook`
    /// - `service_port`: The port that the [`crate::DnetService`] is listening on.
    ///
    /// FIXME: if the same `instance_name` exists, it will be renamed (e.g. `foo` becomes `foo (2)`, `foo (3)` and so on)
    /// so we need to know that and unregister with the correct name.
    pub(super) async fn mdns_register(&self) -> eyre::Result<String> {
        // register your own hostname
        log::debug!("Registering {} of host {}", self.instance, self.hostname);
        let service_info = ServiceInfo::new(
            Self::MDNS_SERVICE_TYPE,
            &self.instance,
            &format!("{}.local.", self.hostname),
            "", // thanks to `enable_addr_auto` we can give this as empty string
            0,  // a dummy port, we will use TXT RECORDs instead
            &self.properties,
        )
        .expect("valid service info")
        // automatically update the addresses of this service, when IP address(es) are added or removed on the host
        .enable_addr_auto();

        // get fullname before consuming the service_info
        let service_fullname = service_info.get_fullname().to_string();

        // register the service with mDNS
        self.mdns
            .register(service_info)
            .expect("Failed to register mDNS service");

        log::info!("Registered service {service_fullname}",);

        Ok(service_fullname)
    }

    /// Unregisters the service from mDNS.
    pub(super) async fn mdns_unregister(&mut self) {
        log::debug!("Unregistering service {}", self.fullname);

        // unregister the service
        match self.mdns.unregister(&self.fullname) {
            Ok(receiver) => {
                while let Ok(status) = receiver.recv() {
                    match status {
                        UnregisterStatus::OK => {
                            log::warn!("Service {} unregistered", self.fullname);
                        }
                        UnregisterStatus::NotFound => {
                            log::error!("Service {} was not registered!", self.fullname);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to unregister service {}: {}", self.fullname, e);
            }
        }
    }

    /// Shutdown the mDNS daemon gracefully.
    pub(super) async fn mdns_shutdown(&self) {
        const RETRY_SLEEP: Duration = Duration::from_millis(200);

        while let Err(err) = self.mdns.shutdown() {
            tokio::time::sleep(RETRY_SLEEP).await;
            if let mdns_sd::Error::Again = err {
                continue;
            } else {
                log::error!("Failed to shutdown mDNS daemon: {err}");
            }
            break;
        }
    }

    /// Updates the service properties for the registered service.
    ///
    /// This is done by re-registering the service with the updated properties,
    /// as noted in [`mdns-sd` documentation](https://docs.rs/mdns-sd/0.13.9/mdns_sd/struct.ServiceDaemon.html#method.register)
    pub(super) async fn mdns_update_service(&self) {
        if let Err(err) = self.mdns_register().await {
            log::error!("Failed to update mDNS service properties: {err}");
        } else {
            log::info!("Updated mDNS service properties for {}", self.fullname);
        }
    }
}
