use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::{io::AsyncWriteExt, net::TcpStream};

const PING_INTERVAL: Duration = Duration::from_secs(5);

impl super::DnetMDNSDameon {
    /// Browses for mDNS services and prints the resolved service information.
    pub async fn browse(&mut self) -> eyre::Result<()> {
        let mdns = ServiceDaemon::new()?;

        log::debug!("Browsing for dnet services");
        let receiver = mdns.browse(Self::SERVICE_TYPE).expect("failed to browse");

        // create an interval to ping discovered services
        let mut ping_interval = tokio::time::interval(PING_INTERVAL);
        ping_interval.tick().await; // wait for the first tick

        loop {
            tokio::select! {
                _ = self.cancellation.cancelled() => break,
                // wait for the next tick
                _ = ping_interval.tick() => {
                    // send a hello message to all services
                    log::debug!("Helloing dnet services");
                    self.handle_pings().await;
                }
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
                                      self.peers.remove(&fullname);
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

    // TODO: we may use this logic to print out topology etc.
    pub async fn handle_pings(&self) {
        for (fullname, addr) in self.peers.iter() {
            if let Ok(mut client) = TcpStream::connect(&addr).await {
                // send hello world
                let msg = b"PING";
                if let Err(err) = client.write_all(msg).await {
                    log::error!(
                        "Could not send message to {} at {}: {}",
                        fullname,
                        addr,
                        err
                    );
                }

                // wait for reply
                if let Err(err) = client.readable().await {
                    log::error!("Could not wait: {err}");
                    continue;
                }

                let mut buf = vec![0; 1024];
                let Ok(n) = client.try_read(&mut buf) else {
                    log::error!("Could not read");
                    continue;
                };
                if n == 0 {
                    log::warn!("Connection closed by peer");
                    continue;
                }
                let data = &buf[..n];
                log::info!("Received data: {:?}", String::from_utf8_lossy(data));
            }
        }
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

                // TODO: parse these
                for prop in info.get_properties().iter() {
                    log::info!(
                        "{}: {}",
                        prop.key(),
                        String::from_utf8_lossy(prop.val().unwrap_or_default())
                    );
                }

                // record peers
                self.peers
                    .insert(info.get_fullname().to_string(), addr_port);
            }
        } else {
            log::trace!("Ignoring service {}", info.get_fullname());
        }
    }
}
