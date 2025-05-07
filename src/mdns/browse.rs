use mdns_sd::{ServiceDaemon, ServiceEvent};

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
              event_res = receiver.recv_async() => {
                  match event_res {
                    Err(e) => {
                      log::error!("Error receiving event: {:?}", e);
                    }
                    Ok(event) => {
                      match event {
                          ServiceEvent::ServiceResolved(info) => {
                              if info.get_fullname().ends_with(Self::SERVICE_TYPE) {
                                log::info!(
                                      "{} resolved at {}:{}",
                                      info.get_fullname(),
                                      info.get_hostname(),
                                      info.get_port(),
                                  );
                                  for addr in info.get_addresses_v4().iter() {
                                      log::info!(" Address: {}", addr);
                                  }

                                  // txt records
                                  // for prop in info.get_properties().iter() {
                                  //     println!(" Property: {}", prop);
                                  // }

                              } else {
                                  log::debug!("Ignoring service {}", info.get_fullname());
                              }
                          },
                          ServiceEvent::ServiceRemoved(service_name, fullname) => {
                              log::warn!("Service {service_name} removed: {fullname}");
                          },
                          other_event => {
                              log::trace!("{:?}", other_event);
                          }
                      }
                  }
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
}
