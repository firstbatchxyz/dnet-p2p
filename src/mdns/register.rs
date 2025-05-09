use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceInfo, UnregisterStatus};

impl super::DnetMDNSDameon {
    /// Registers a service with the given instance name and hostname.
    ///
    /// - `instance_name`: The name of the service instance, e.g. `dnet1`
    /// - `hostname`: The hostname of the service, e.g. `john-doe-macbook`
    /// - `service_port`: The port that the [`crate::DnetService`] is listening on.
    pub async fn register(
        &self,
        instance_name: &str,
        hostname: &str,
        service_port: u16,
    ) -> eyre::Result<()> {
        let mdns = ServiceDaemon::new()?;

        // check your own hostname if it exists
        // if let Ok(HostnameResolutionEvent::AddressesFound(existing_hostname, addrs)) =
        //     mdns.resolve_hostname(&service_hostname, None)?.recv()
        // {
        //     println!(
        //         "Host {} already exists at {:?}, unregistering.",
        //         existing_hostname, addrs
        //     );
        //     mdns.stop_resolve_hostname(&service_hostname).unwrap();
        // } else {
        //     mdns.stop_resolve_hostname(&service_hostname).unwrap();

        // register your own hostname
        log::debug!("Registering {instance_name} of host {hostname}");
        let service_info = ServiceInfo::new(
            Self::SERVICE_TYPE,
            instance_name,
            &format!("{}.local.", hostname),
            "", // thanks to `enable_addr_auto` we can give this as empty string
            service_port,
            &self.properties,
        )
        .expect("valid service info")
        // automatically update the addresses of this service, when IP address(es) are added or removed on the host
        .enable_addr_auto();

        let service_fullname = service_info.get_fullname().to_string();
        mdns.register(service_info)
            .expect("Failed to register mDNS service");

        log::info!("Registered service {service_fullname}",);

        // monitor the daemon for events
        let monitor = mdns.monitor().expect("Failed to monitor the daemon");
        loop {
            tokio::select! {
              // monitor mdns events
              event_opt = monitor.recv_async() => {
                  match event_opt {
                      Ok(event) => {
                          match event {
                            DaemonEvent::Announce(service, interface) => {
                                log::debug!("Service {service} announced at {interface}");
                            },
                            DaemonEvent::Error(err) => {
                                log::error!("Daemon error: {}", err);
                            },
                            other => {
                                log::trace!("Daemon event: {:?}", other);
                            }
                          };
                      },
                      Err(e) => {
                          log::error!("Error receiving event: {:?}", e);
                          break;
                      }
                };
              },
              // shutdown daemon
              _ = self.cancellation.cancelled() => {
                log::debug!("Cancellation signal received, unregistering service");
                  let receiver = mdns.unregister(&service_fullname).unwrap();
                  while let Ok(status) = receiver.recv() {
                      match status {
                        UnregisterStatus::OK => {
                          log::warn!("Service {service_fullname} unregistered");
                          return Ok(());
                        }
                        UnregisterStatus::NotFound => {
                          log::warn!("Service {service_fullname} not found!");
                          return Ok(());
                        }
                      }
                  }
              },

            }
        }

        if let Ok(status) = mdns.shutdown().unwrap().recv() {
            log::debug!("Daemon status: {:?}", status);
        }

        Ok(())
    }
}
