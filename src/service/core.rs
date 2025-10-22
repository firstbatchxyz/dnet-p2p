use eyre::Context;
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

use super::{
    udp::{UdpDiscovery, UdpMessage},
    DnetServiceProperties,
};
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
    /// Whether this service is passive (monitors only, doesn't register).
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

        log::info!(
            "Starting UDP discovery loop on port {} for instance {}",
            self.udp.port,
            self.properties.instance
        );
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

              // receive peer messages (update or remove)
              message = self.udp.receive_announcement() => {
                  match message {
                      Ok(Some((msg, sender_addr))) => {
                          self.handle_udp_message(msg, sender_addr);
                      },
                      Ok(None) => {
                          // no data available, continue
                      },
                      Err(e) => {
                          log::error!("Error receiving UDP message: {e}");
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

    /// Handles a UDP message from a peer (Update or Remove)
    #[inline]
    fn handle_udp_message(&mut self, message: UdpMessage, sender_addr: std::net::SocketAddr) {
        match message {
            UdpMessage::Update(properties) => {
                let instance = properties.instance.clone();

                // check if this is us (only relevant if not passive)
                if !self.is_passive && instance == self.properties.instance {
                    log::debug!("Received our own announcement from {}", sender_addr);
                    // update yourself in peer props
                    self.peer_props
                        .insert(instance.clone(), self.properties.clone());
                    return;
                }

                log::debug!("Processing UPDATE from {}: {:#?}", instance, properties);

                // check if we are both a manager
                if self.is_manager && properties.is_manager {
                    log::error!("Found another manager {instance} at {sender_addr}, ignoring...");
                } else {
                    // add peer to the peer properties
                    self.peer_props.insert(instance.clone(), properties);
                    log::info!("Added/updated peer: {}", instance);
                }
            }
            UdpMessage::Remove(instance) => {
                log::info!("Processing REMOVE for instance: {}", instance);

                // check if we know about this peer
                if let Some(stored_props) = self.peer_props.get(&instance) {
                    let sender_ip = sender_addr.ip().to_string();

                    // verify that the sender IP matches the stored peer IP for sanity
                    if stored_props.local_ip == sender_ip {
                        self.peer_props.remove(&instance);
                        log::info!("Removed {} (from: {})", instance, sender_ip);
                    } else {
                        log::warn!(
                            "Ignoring REMOVE due to IP mismatch {} (from {}, expected IP: {})",
                            instance,
                            sender_ip,
                            stored_props.local_ip
                        );
                    }
                } else {
                    log::warn!(
                        "Received REMOVE for unknown peer {} (from {})",
                        instance,
                        sender_addr
                    );
                }
            }
        }
    }

    /// Cancels the token, to gracefully stop the service.
    pub fn trigger_cancellation(&self) {
        self.cancellation.cancel();
    }

    /// Stops the service gracefully.
    pub async fn stop(&mut self) {
        log::info!("Stopping service gracefully...");

        // broadcast removal notification if not passive
        if !self.is_passive {
            log::info!(
                "Broadcasting REMOVE notification for instance {}",
                self.properties.instance
            );
            if let Err(e) = self.udp.broadcast_remove(&self.properties.instance).await {
                log::error!("Failed to broadcast removal notification: {e}");
            } else {
                log::info!("REMOVE notification sent successfully");
            }
            // delay to ensure the removal message is sent before we close the socket
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }

    /// Sets the busy status of the service.
    pub async fn set_is_busy(&mut self, is_busy: bool) {
        // handle no-ops & passive
        if self.properties.is_busy == is_busy || self.is_passive {
            return;
        }
        self.properties.is_busy = is_busy;

        // property will be broadcast on next interval
        // TODO: do the broadcast immediately maybe?
        log::debug!("Set is_busy to {is_busy}");
    }
}
