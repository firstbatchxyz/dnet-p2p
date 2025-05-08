use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

impl super::DnetMDNSDameon {
    /// Browses for mDNS services and prints the resolved service information.
    pub async fn browse(&self) -> eyre::Result<()> {
        let mdns = ServiceDaemon::new()?;

        // brwose the service
        log::debug!("Browsing for dnet services");
        let receiver = mdns.browse(Self::SERVICE_TYPE).expect("failed to browse");
        loop {
            tokio::select! {
                _ = self.cancellation.cancelled() => break,
                // check for new events
                event_res = receiver.recv_async() => {
                    match event_res {
                        Ok(event) => {
                            match event {
                                ServiceEvent::ServiceResolved(info) => {
                                    self.handle_service_resolved(info);

                                },
                                ServiceEvent::ServiceRemoved(service_name, fullname) => {
                                    log::warn!("Service {service_name} removed: {fullname}");
                                },
                                other_event => {
                                    log::trace!("{:?}", other_event);
                                }
                            }
                        },
                        Err(err) => log::error!("Error receiving event: {err}"),
                    };
              }
            }
        }

        log::info!("Shutting down daemon.");
        mdns.stop_browse(Self::SERVICE_TYPE).unwrap();
        if let Ok(status) = mdns.shutdown().unwrap().recv() {
            println!("Daemon status: {:?}", status);
        }

        Ok(())
    }

    pub fn handle_service_resolved(&self, info: ServiceInfo) {
        if info.get_fullname().ends_with(Self::SERVICE_TYPE) {
            if let Some(addr) = info
                .get_addresses_v4()
                .iter()
                .filter(|addr| addr.is_private())
                .next()
            {
                log::info!(
                    "{} resolved at host {} listening on {}:{}",
                    info.get_fullname(),
                    info.get_hostname(),
                    addr,
                    info.get_port(),
                );

                // TODO: parse these
                for prop in info.get_properties().iter() {
                    log::info!(
                        "{}: {}",
                        prop.key(),
                        String::from_utf8_lossy(prop.val().unwrap_or_default())
                    );
                }
            }
        } else {
            log::trace!("Ignoring service {}", info.get_fullname());
        }
    }
}
