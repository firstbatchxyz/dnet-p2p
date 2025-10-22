use eyre::Context;
// use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent};
use std::{collections::HashMap, env};
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
    // /// mDNS service daemon.
    // pub(crate) mdns: ServiceDaemon,

    /// Name of the host this service is running on.
    ///
    /// Usually retrieved from `gethostname` syscall,
    /// or provided manually.
    pub(crate) hostname: String,
    /// The full name of the service.
    pub(crate) fullname: String,
    /// Whether this service is a manager or not.
    pub(crate) is_manager: bool,
    /// Whether this service is passive (monitors only, doesn't register to mDNS).
    pub(crate) is_passive: bool,
    /// How often to refresh the service properties and update mDNS.
    pub(crate) refresh_timeout: std::time::Duration,
}

impl DnetService {
    pub async fn new(
        cancellation: CancellationToken,
        instance: String,
        hostname: String,
        host: String,
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
            host.clone(),
            server_port,
            shard_port,
            local_ip.to_string(),
        );

        // refresh timeout from env or default
        let refresh_timeout = env::var("DNET_P2P_REFRESH_TIMEOUT")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .map(std::time::Duration::from_secs)
            .unwrap_or(std::time::Duration::from_secs(3));

        // let mdns = ServiceDaemon::new().wrap_err("failed to create mDNS service daemon")?;
        let udp = UdpDiscovery::new()
            .await
            .wrap_err("failed to create UDP discovery")?;

        Ok(Self {
            cancellation,
            peer_props: HashMap::new(),
            properties,
            udp,
            // mdns,
            hostname,
            is_manager,
            is_passive,
            fullname: String::new(), // will be set after registration
            refresh_timeout,
        })
    }
    /// Starts the service with UDP discovery.
    pub async fn start(&mut self) -> eyre::Result<()> {
        // create intervals for UDP operations
        let mut broadcast_interval = tokio::time::interval(UdpDiscovery::get_broadcast_interval());
        broadcast_interval.tick().await; // wait for the first tick

        let mut cleanup_interval = tokio::time::interval(UdpDiscovery::get_cleanup_interval());
        cleanup_interval.tick().await; // wait for the first tick

        // set fullname to instance (for compatibility)
        if !self.is_passive {
            self.fullname = self.properties.instance.clone();
            log::info!("Service instance: {}", self.fullname);
        }

        log::info!("Starting UDP discovery loop...");

        loop {
            tokio::select! {
              // broadcast own properties
              _ = broadcast_interval.tick() => {
                  if !self.is_passive && self.udp.is_enabled() {
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

    // /// Handles monitored mDNS events.
    // #[inline]
    // async fn handle_monitor_event(&mut self, event: DaemonEvent) {
    //     match event {
    //         DaemonEvent::Announce(service, interface) => {
    //             log::debug!("Service {service} announced at {interface}");
    //         }
    //         DaemonEvent::Error(err) => {
    //             log::error!("Daemon error: {err}");
    //         }
    //         DaemonEvent::NameChange(name_change) => {
    //             let old_name = name_change.original.as_str();
    //             let new_name = name_change.new_name.as_str();
    //             log::debug!("Service name change: {old_name} -> {new_name}");

    //             // if our service name was changed due to conflict, update our fullname
    //             if old_name == self.fullname {
    //                 log::warn!(
    //                     "Service name changed from {old_name} to {new_name} due to conflict"
    //                 );
    //                 self.fullname = new_name.to_string();

    //                 // also update our own entry in peer_props if it exists
    //                 if let Some(props) = self.peer_props.remove(old_name) {
    //                     self.peer_props.insert(new_name.to_string(), props);
    //                 }
    //             }
    //         }
    //         other => {
    //             log::debug!("Daemon event: {other:?}");
    //         }
    //     };
    // }

    // /// Handles an mDNS service browse event.
    // ///
    // /// If a service for `dnet` is resolved, it will be added to the list of known peers to this service.
    // #[inline]
    // fn handle_browse_event(&mut self, event: ServiceEvent) {
    //     match event {
    //         ServiceEvent::ServiceResolved(info) => {
    //             let fullname = info.get_fullname();

    //             // ignore non-dnet services
    //             if !fullname.ends_with(Self::MDNS_SERVICE_TYPE) {
    //                 log::trace!("Ignoring service {fullname}, not a dnet service");
    //                 return;
    //             }

    //             // check if this is us (only relevant if not passive)
    //             if !self.is_passive && fullname == self.fullname {
    //                 log::debug!("Resolved our own service: {fullname}");

    //                 // update yourself in peer props, this is to "act" like you discovered
    //                 // yourself, even if the props were updated anyways
    //                 self.peer_props
    //                     .insert(fullname.to_string(), self.properties.clone());
    //                 return;
    //             }

    //             if let Some(addr) = info
    //                 .get_addresses_v4()
    //                 .iter()
    //                 // get the first address that is private (belongs to the local network)
    //                 .find(|addr| addr.is_private())
    //             {
    //                 log::info!(
    //                     "{fullname} resolved at host {} listening on {addr}",
    //                     info.get_hostname(),
    //                 );

    //                 let properties = match DnetServiceProperties::try_from(info.get_properties()) {
    //                     Ok(props) => props,
    //                     Err(e) => {
    //                         log::error!(
    //                             "Failed to parse properties for {}: {e}",
    //                             info.get_fullname()
    //                         );
    //                         return;
    //                     }
    //                 };
    //                 log::debug!("{properties:#?}");

    //                 // check if we are both a manager
    //                 if self.is_manager && properties.is_manager {
    //                     log::error!("Found another manager {fullname}, ignoring...",);
    //                 } else {
    //                     // add peer to the peer properties
    //                     self.peer_props.insert(fullname.to_string(), properties);
    //                 }
    //             }
    //         }
    //         ServiceEvent::ServiceRemoved(service_name, fullname) => {
    //             if service_name.ends_with(Self::MDNS_SERVICE_TYPE) {
    //                 log::warn!("Service {service_name} removed: {fullname}");
    //                 self.peer_props.remove(fullname.as_str());
    //             }
    //         }
    //         event => log::trace!("{event:?}"),
    //     }
    // }

    /// Handles a UDP announcement from a peer
    #[inline]
    fn handle_udp_announcement(
        &mut self,
        properties: DnetServiceProperties,
        sender_addr: std::net::SocketAddr,
    ) {
        let instance = properties.instance.clone();

        // check if this is us (only relevant if not passive)
        if !self.is_passive && instance == self.fullname {
            log::debug!("Received our own announcement from {}", sender_addr);
            // update yourself in peer props
            self.peer_props
                .insert(instance.clone(), self.properties.clone());
            return;
        }

        log::debug!("Processing announcement from {}: {:#?}", instance, properties);

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

    // /// Updates the properties on mDNS.
    // #[inline]
    // async fn handle_refresh_and_update(&mut self) -> eyre::Result<()> {
    //     // only update mDNS service if not passive
    //     if !self.is_passive {
    //         self.mdns_update_service().await;
    //     }

    //     Ok(())
    // }

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
