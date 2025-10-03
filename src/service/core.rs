use eyre::Context;
use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;

use super::DnetServiceProperties;
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
    /// This is published via mDNS to all other services.
    pub(crate) properties: DnetServiceProperties,
    /// mDNS service daemon.
    pub(crate) mdns: ServiceDaemon,

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
}

impl DnetService {
    pub fn new(
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

        let mdns = ServiceDaemon::new().wrap_err("failed to create mDNS service daemon")?;
        Ok(Self {
            cancellation,
            peer_props: HashMap::new(),
            properties,
            mdns,
            hostname,
            is_manager,
            is_passive,
            fullname: String::new(), // will be set after registration
        })
    }
    /// Starts the service with mDNS daemon.
    pub async fn start(&mut self) -> eyre::Result<()> {
        // create an interval to refresh properties
        const PROPERTY_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(120);
        let mut property_refresh_interval = tokio::time::interval(PROPERTY_REFRESH_INTERVAL);
        property_refresh_interval.tick().await; // wait for the first tick

        // only register to mDNS if not passive
        if !self.is_passive {
            self.fullname = self.mdns_register().await?;
        }

        // browse for services
        let browser = self
            .mdns
            .browse(Self::MDNS_SERVICE_TYPE)
            .wrap_err("failed to browse services")?;

        // monitor the daemon for events
        let monitor = self.mdns.monitor().wrap_err("could not monitor mdns")?;

        loop {
            tokio::select! {
              // handle property refresh
              _ = property_refresh_interval.tick() => {
                  if let Err(e) = self.handle_property_refresh().await {
                      log::error!("Failed to refresh properties: {e}");
                  }
              },

              // monitor browse events
              event_res = browser.recv_async() => {
                  match event_res {
                      Ok(event) => self.handle_browse_event(event),
                      Err(err) => log::error!("Error receiving event: {err}"),
                  };
              },

              // monitor mDNS events
              event_res = monitor.recv_async() => {
                  match event_res {
                      Ok(event) => self.handle_monitor_event(event).await,
                      Err(err) => log::error!("Error receiving event: {err}"),
                };
              },

              // graceful shutdown
              _ = self.cancellation.cancelled() => break,
            }
        }

        self.stop().await;
        Ok(())
    }

    /// Handles monitored mDNS events.
    #[inline]
    async fn handle_monitor_event(&self, event: DaemonEvent) {
        match event {
            DaemonEvent::Announce(service, interface) => {
                log::debug!("Service {service} announced at {interface}");
            }
            DaemonEvent::Error(err) => {
                log::error!("Daemon error: {err}");
            }
            other => {
                log::trace!("Daemon event: {other:?}");
            }
        };
    }

    /// Handles an mDNS service browse event.
    ///
    /// If a service for `dnet` is resolved, it will be added to the list of known peers to this service.
    #[inline]
    fn handle_browse_event(&mut self, event: ServiceEvent) {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let fullname = info.get_fullname();

                // ignore non-dnet services
                if !fullname.ends_with(Self::MDNS_SERVICE_TYPE) {
                    log::trace!("Ignoring service {fullname}, not a dnet service");
                    return;
                }

                // check if this is us (only relevant if not passive)
                if !self.is_passive && fullname == self.fullname {
                    log::debug!("Resolved our own service: {fullname}");

                    // update yourself in peer props, this is to "act" like you discovered
                    // yourself, even if the props were updated anyways
                    self.peer_props
                        .insert(fullname.to_string(), self.properties.clone());
                    return;
                }

                if let Some(addr) = info
                    .get_addresses_v4()
                    .iter()
                    // get the first address that is private (belongs to the local network)
                    .find(|addr| addr.is_private())
                {
                    log::info!(
                        "{fullname} resolved at host {} listening on {addr}",
                        info.get_hostname(),
                    );

                    let properties = match DnetServiceProperties::try_from(info.get_properties()) {
                        Ok(props) => props,
                        Err(e) => {
                            log::error!(
                                "Failed to parse properties for {}: {e}",
                                info.get_fullname()
                            );
                            return;
                        }
                    };
                    log::debug!("{properties:#?}");

                    // check if we are both a manager
                    if self.is_manager && properties.is_manager {
                        log::error!("Found another manager {fullname}, ignoring...",);
                    } else {
                        // add peer to the peer properties
                        self.peer_props.insert(fullname.to_string(), properties);
                    }
                }
            }
            ServiceEvent::ServiceRemoved(service_name, fullname) => {
                if service_name.ends_with(Self::MDNS_SERVICE_TYPE) {
                    log::warn!("Service {service_name} removed: {fullname}");
                    self.peer_props.remove(fullname.as_str());
                }
            }
            event => log::trace!("{event:?}"),
        }
    }

    /// Cancels the token, to gracefully stop the service.
    pub fn trigger_cancellation(&self) {
        self.cancellation.cancel();
    }

    /// Stops the service gracefully.
    pub async fn stop(&mut self) {
        // only unregister if not passive (since we never registered)
        if !self.is_passive {
            self.mdns_unregister().await;
        }

        self.mdns_shutdown().await;
    }

    /// Updates the properties on mDNS.
    #[inline]
    async fn handle_property_refresh(&mut self) -> eyre::Result<()> {
        // only update mDNS service if not passive
        if !self.is_passive {
            self.mdns_update_service().await;
        }

        Ok(())
    }

    /// Sets the busy status of the service and updates mDNS.
    pub async fn set_is_busy(&mut self, is_busy: bool) {
        // handle no-ops & passive
        if self.properties.is_busy == is_busy || self.is_passive {
            return;
        }
        self.properties.is_busy = is_busy;
        self.mdns_update_service().await;

        log::debug!("Set is_busy to {is_busy}");
    }
}
