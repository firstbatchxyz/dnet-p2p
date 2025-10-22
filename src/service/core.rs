use eyre::Context;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

use super::{udp::UdpDiscovery, DnetServiceProperties};
use crate::utils::get_local_network_ip;

/// A `dnet` service.
///
/// Each service has a TCP listener, and a local pool of peers that they are connected to.
pub struct DnetService {
    /// The cancellation token to cancel the service gracefully.
    ///
    /// Usually listens to CTRL+C, or any other graceful shutdown on errors.
    pub cancellation: CancellationToken,
    /// A mapping of services from their `fullname` to their last-seen properties.
    pub peer_props: HashMap<String, DnetServiceProperties>,
    /// A shared service properties object.
    ///
    /// This is published via UDP to all other services.
    pub(crate) properties: DnetServiceProperties,
    /// UDP discovery instance.
    pub(crate) udp: UdpDiscovery,
    /// Whether this service is a manager or not.
    pub(crate) is_manager: bool,
    /// Whether this service is passive (monitors only, doesn't register to mDNS).
    pub(crate) is_passive: bool,
}

impl DnetService {
    pub async fn new(
        cancellation: CancellationToken,
        instance: String,
        server_port: u16,
        shard_port: u16,
        is_manager: bool,
        is_passive: bool,
    ) -> eyre::Result<Self> {
        let local_ip = match get_local_network_ip() {
            Some((interface, ip)) => {
                log::info!("Using local network IP address via {interface}: {ip}");
                ip
            }
            None => {
                eyre::bail!("Could not determine local network IP address")
            }
        };

        let properties = DnetServiceProperties::new(
            is_manager,
            instance.clone(),
            server_port,
            shard_port,
            local_ip.to_string(),
        );

        let udp = UdpDiscovery::new()
            .await
            .wrap_err("failed to create UDP discovery")?;

        Ok(Self {
            cancellation,
            peer_props: HashMap::new(),
            properties,
            udp,
            is_manager,
            is_passive,
        })
    }
    /// Starts the service with UDP discovery.
    pub async fn start(&mut self) -> eyre::Result<()> {
        // create intervals for UDP operations
        let mut broadcast_interval = tokio::time::interval(self.udp.broadcast_interval);
        broadcast_interval.tick().await; // wait for the first tick

        let mut cleanup_interval = tokio::time::interval(self.udp.cleanup_interval);
        cleanup_interval.tick().await; // wait for the first tick

        log::info!("Starting UDP discovery loop...");
        loop {
            tokio::select! {
              // broadcast own properties
              _ = broadcast_interval.tick() => {
                  if !self.is_passive {
                      if let Err(e) = self.udp.broadcast_properties(&self.properties).await {
                          log::error!("Failed to broadcast properties: {e}");
                      }
                  }
              },

              // cleanup stale peers
              _ = cleanup_interval.tick() => {
                  let timed_out = self.udp.cleanup_stale_peers();
                  for instance in timed_out {
                      self.peer_props.remove(&instance);
                      log::info!("Removed stale peer: {}", instance);
                  }
              },

              // receive peer announcements
              announcement = self.udp.receive_announcement() => {
                  match announcement {
                      Ok(Some((properties, sender_addr))) => {
                          self.handle_udp_announcement(properties, sender_addr);
                      },
                      Ok(None) => {
                          // no data available, continue
                      },
                      Err(e) => {
                          log::error!("Error receiving UDP announcement: {e}");
                      }
                  }
              },

              // graceful shutdown
              _ = self.cancellation.cancelled() => break,
            }
        }

        self.stop().await;
        Ok(())
    }

    /// Handles a UDP announcement from a peer
    #[inline]
    fn handle_udp_announcement(
        &mut self,
        properties: DnetServiceProperties,
        sender_addr: std::net::SocketAddr,
    ) {
        let instance = properties.instance.clone();

        // check if this is us (only relevant if not passive)
        if !self.is_passive && instance == self.properties.instance {
            log::debug!("Received our own announcement from {}", sender_addr);
            // update yourself in peer props
            self.peer_props
                .insert(instance.clone(), self.properties.clone());
            return;
        }

        log::debug!(
            "Processing announcement from {}: {:#?}",
            instance,
            properties
        );

        // check if we are both a manager
        if self.is_manager && properties.is_manager {
            log::error!("Found another manager {instance} at {sender_addr}, ignoring...");
        } else {
            // add peer to the peer properties
            self.peer_props.insert(instance.clone(), properties);
            log::info!("Added/updated peer: {}", instance);
        }
    }

    /// Cancels the token, to gracefully stop the service.
    pub fn trigger_cancellation(&self) {
        self.cancellation.cancel();
    }

    /// Stops the service gracefully.
    pub async fn stop(&mut self) {
        log::info!("Stopping service gracefully...");
        // UDP doesn't require explicit shutdown, socket will be dropped
        // // only unregister if not passive (since we never registered)
        // if !self.is_passive {
        //     self.mdns_unregister().await;
        // }
        // self.mdns_shutdown().await;
    }

    /// Sets the busy status of the service.
    pub async fn set_is_busy(&mut self, is_busy: bool) {
        // handle no-ops & passive
        if self.properties.is_busy == is_busy || self.is_passive {
            return;
        }
        self.properties.is_busy = is_busy;
        // Property will be broadcast on next interval
        log::debug!("Set is_busy to {is_busy}");
    }
}
