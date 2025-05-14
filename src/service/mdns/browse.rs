use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::ServiceProperties;

impl crate::DnetService {
    /// Browses for mDNS services and prints the resolved service information.
    pub async fn browse(&mut self) -> eyre::Result<()> {
        let mdns = ServiceDaemon::new()?;

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
                                    if service_name.ends_with(Self::SERVICE_TYPE) {
                                      log::warn!("Service {service_name} removed: {fullname}");
                                      self.peer_props.remove(fullname.as_str());
                                      self.peer_conns.remove(fullname.as_str());
                                    }
                                },
                                event => log::trace!("{event:?}"),
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

    pub fn handle_service_resolved(&mut self, info: ServiceInfo) {
        if info.get_fullname().ends_with(Self::SERVICE_TYPE) {
            if let Some(addr) = info
                .get_addresses_v4()
                .iter()
                // get the first address that is private (belongs to the local network)
                .filter(|addr| addr.is_private())
                .next()
            {
                let addr_port = format!("{}:{}", addr, info.get_port());
                log::info!(
                    "{} resolved at host {} listening on {addr_port}",
                    info.get_fullname(),
                    info.get_hostname(),
                );

                let properties = ServiceProperties::from(info.get_properties().clone());

                // record service
                self.peer_props
                    .insert(info.get_fullname().to_string(), properties);

                //
            }
        } else {
            log::trace!("Ignoring service {}", info.get_fullname());
        }
    }
}
