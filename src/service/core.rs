use eyre::Context;
use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent, UnregisterStatus};
use std::{
    collections::HashMap,
    net::{SocketAddr, SocketAddrV4},
    str::FromStr,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

use super::ServiceProperties;

/// A `dnet` service.
///
/// Each service has a TCP listener, and a local pool of peers that they are connected to.
pub struct DnetService {
    /// The cancellation token to cancel the service gracefully.
    ///
    /// Usually listens to CTRL+C, or any other graceful shutdown on errors.
    pub(crate) cancellation: CancellationToken,
    /// An active TCP listener that accepts incoming connections.
    pub(crate) listener: TcpListener,
    /// Actively listening port.
    pub(crate) port: u16,
    /// A mapping of services from their `fullname` to their last-seen properties.
    pub(crate) peer_props: HashMap<String, (ServiceProperties, SocketAddrV4)>,
    /// A system information object to monitor resources.
    pub(crate) sysinfo: sysinfo::System,
    /// A shared service properties object.
    ///
    /// This is published via mDNS to all other services.
    pub(crate) properties: ServiceProperties,
    /// mDNS service daemon.
    pub(crate) mdns: ServiceDaemon,

    pub(crate) instance_name: String,
    pub(crate) hostname: String,
    /// The full name of the service.
    pub(crate) fullname: String,
    /// Whether this service is a manager or not.
    pub(crate) is_manager: bool,
}

impl DnetService {
    pub async fn new(
        cancellation: CancellationToken,
        instance_name: String,
        hostname: String,
        port: Option<u16>,
        is_manager: bool,
    ) -> eyre::Result<Self> {
        let sysinfo = sysinfo::System::new_all();

        // listen at the given port, or a random one
        let listen_addr = SocketAddr::V4(SocketAddrV4::from_str(&format!(
            "0.0.0.0:{}",
            port.unwrap_or(0)
        ))?);

        let properties = ServiceProperties::new(&sysinfo, is_manager);
        let listener = TcpListener::bind(listen_addr)
            .await
            .wrap_err("failed to bind to TCP listener")?;

        // if the port is not given, get the local address and extract the ports
        let port = port.unwrap_or_else(|| {
            listener
                .local_addr()
                .map(|a| a.port())
                .expect("could not get local address")
        });

        Ok(Self {
            cancellation,
            listener,
            port,
            sysinfo,
            peer_props: HashMap::new(),
            properties,
            mdns: ServiceDaemon::new().wrap_err("failed to create mDNS service daemon")?,
            instance_name,
            hostname,
            fullname: String::new(),
            is_manager,
        })
    }
    /// Starts the service as a worker and listens for incoming connections.
    pub async fn start(&mut self) -> eyre::Result<()> {
        // create an interval to refresh properties
        const PROPERTY_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
        let mut property_refresh_interval = tokio::time::interval(PROPERTY_REFRESH_INTERVAL);
        property_refresh_interval.tick().await; // wait for the first tick

        self.fullname = self.register().await?;

        // browse for services
        let browser = self
            .mdns
            .browse(Self::MDNS_SERVICE_TYPE)
            .wrap_err("failed to browse services")?;

        // monitor the daemon for events
        let monitor = self.mdns.monitor().wrap_err("could not monitor mdns")?;

        loop {
            tokio::select! {
              // handle incoming connections
              accept_result = self.listener.accept() => {
                  match accept_result {
                      Ok((connection, socket)) => self.handle_connection(connection, socket).await,
                      Err(err) => log::error!("Failed to accept connection: {err}"),
                  }
              }

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

              // monitor mDNS events if we are not a manager
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
                log::error!("Daemon error: {}", err);
            }
            other => {
                log::trace!("Daemon event: {:?}", other);
            }
        };
    }

    #[inline]
    fn handle_browse_event(&mut self, event: ServiceEvent) {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                if info.get_fullname().ends_with(Self::MDNS_SERVICE_TYPE) {
                    if let Some(addr) = info
                        .get_addresses_v4()
                        .iter()
                        // get the first address that is private (belongs to the local network)
                        .filter(|addr| addr.is_private())
                        .next()
                    {
                        log::info!(
                            "{} resolved at host {} listening on {addr}",
                            info.get_fullname(),
                            info.get_hostname(),
                        );

                        let addr = SocketAddrV4::from_str(&format!("{}:{}", addr, info.get_port()))
                            .expect("should parse ipv4 address");
                        let properties = ServiceProperties::from(info.get_properties());
                        log::debug!("{properties:#?}");

                        // check if we are both a manager
                        if self.is_manager && properties.is_manager {
                            log::error!(
                                "Found another manager {} at {addr}, ignoring...",
                                info.get_fullname()
                            );
                        } else {
                            self.peer_props
                                .insert(info.get_fullname().to_string(), (properties, addr));
                        }
                    }
                } else {
                    log::trace!("Ignoring service {}", info.get_fullname());
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

    /// Stops the service gracefully.
    async fn stop(&mut self) {
        // unregister the service
        match self.mdns.unregister(&self.fullname) {
            Ok(receiver) => {
                while let Ok(status) = receiver.recv() {
                    match status {
                        UnregisterStatus::OK => {
                            log::warn!("Service {} unregistered", self.fullname);
                        }
                        UnregisterStatus::NotFound => {
                            log::warn!("Service {} not found!", self.fullname);
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to unregister service {}: {}", self.fullname, e);
            }
        }

        // shutdown the mdns daemon
        while let Err(e) = self.mdns.shutdown() {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if let mdns_sd::Error::Again = e {
                continue;
            } else {
                log::error!("Failed to shutdown mDNS daemon: {}", e);
            }
            break;
        }
    }

    /// Refreshes the system information and updates the properties.
    #[inline]
    async fn handle_property_refresh(&mut self) -> eyre::Result<()> {
        self.sysinfo.refresh_all();
        self.properties.refresh_sysinfo(&self.sysinfo);

        // TODO: how to update the properties?
        // or should we use the TCP connections for that
        // self.register().await?;
        Ok(())
    }

    #[inline]
    async fn handle_connection(&mut self, _connection: TcpStream, socket: SocketAddr) {
        log::info!("Accepted connection from {}", socket);
        // TODO: exchange peer info here, so that we can record the fullname
    }
}
