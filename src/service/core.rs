use eyre::Context;
use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent, ServiceInfo, UnregisterStatus};
use std::{collections::HashMap, net::SocketAddr};
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
    /// A mapping of services from their `fullname` to their established connections.
    /// TODO: do we need this?
    // pub(crate) peer_conns: HashMap<String, TcpStream>,
    /// A mapping of services from their `fullname` to their last-seen properties.
    pub(crate) peer_props: HashMap<String, ServiceProperties>,
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
    /// Indicates that this service is a manager.
    pub(crate) is_manager: bool,
}

impl DnetService {
    pub async fn new(
        cancellation: CancellationToken,
        instance_name: String,
        hostname: String,
        port: Option<u16>,
    ) -> eyre::Result<Self> {
        // listen at the given port, or a random one
        let listen_addr = format!("0.0.0.0:{}", port.unwrap_or(0));
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
            sysinfo: sysinfo::System::new_all(),
            // peer_conns: HashMap::new(),
            peer_props: HashMap::new(),
            properties: Default::default(),
            mdns: ServiceDaemon::new().wrap_err("failed to create mDNS service daemon")?,
            instance_name,
            hostname,
            fullname: String::new(), // TODO: !!!
            is_manager: false,
        })
    }

    #[inline(always)]
    pub fn as_manager(mut self) -> Self {
        self.is_manager = true;
        self
    }

    /// Starts the service as a worker and listens for incoming connections.
    pub async fn start(&mut self) -> eyre::Result<()> {
        // create an interval to refresh properties
        const PROPERTY_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
        let mut property_refresh_interval = tokio::time::interval(PROPERTY_REFRESH_INTERVAL);
        property_refresh_interval.tick().await; // wait for the first tick

        self.fullname = self.register().await?;

        // FIXME: this is very smelly, dont interleave two types like this

        // monitor the daemon for events
        // FIXME: this does not work
        let monitor = if !self.is_manager {
            Some(self.mdns.monitor().wrap_err("could not monitor mdns")?)
        } else {
            None
        };
        // if we are a manager, we need to browse for services
        // FIXME: this does not work
        let browser = if self.is_manager {
            Some(
                self.mdns
                    .browse(Self::MDNS_SERVICE_TYPE)
                    .wrap_err("failed to browse")?,
            )
        } else {
            None
        };

        loop {
            let is_manager = self.is_manager;
            tokio::select! {
                // handle incoming connections
                accept_result = self.listener.accept() => {
                match accept_result {
                  Ok((connection, socket)) => {
                    self.handle_connection(connection, socket).await;
                  }
                  Err(err) => {
                    log::error!("Failed to accept connection: {err}");
                  }
                }
              }
              // handle property refresh
              _ = property_refresh_interval.tick() => {
                  if let Err(e) = self.handle_property_refresh().await {
                      log::error!("Failed to refresh properties: {e}");
                  }
              },
              // monitor mDNS events if we are not a manager
              // FIXME: this does not work
              event_res = monitor.as_ref().unwrap().recv_async(), if !is_manager => {
                  match event_res {
                      Ok(event) => {
                          self.handle_worker_monitor(event).await;
                      },
                      Err(e) => {
                          log::error!("Error receiving event: {:?}", e);
                          break;
                      }
                };
              },
              // monitor browse events if we are a manager
              // FIXME: this does not work
               event_res = browser.as_ref().unwrap().recv_async(), if is_manager => {
                    match event_res {
                        Ok(event) => {
                            match event {
                                ServiceEvent::ServiceResolved(info) => {
                                    self.handle_service_resolved(info);
                                },
                                ServiceEvent::ServiceRemoved(service_name, fullname) => {
                                    if service_name.ends_with(Self::MDNS_SERVICE_TYPE) {
                                      log::warn!("Service {service_name} removed: {fullname}");
                                      self.peer_props.remove(fullname.as_str());
                                      // self.peer_conns.remove(fullname.as_str());
                                    }
                                },
                                event => log::trace!("{event:?}"),
                            }
                        },
                        Err(err) => log::error!("Error receiving event: {err}"),
                    };
              }
              // handle cancellation
              _ = self.cancellation.cancelled() => break,
            }
        }

        self.stop().await;
        Ok(())
    }

    /// Handles monitored mDNS events.
    #[inline]
    async fn handle_worker_monitor(&self, event: DaemonEvent) {
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
    /// Stops the service gracefully.
    async fn stop(&mut self) {
        let receiver = self.mdns.unregister(&self.fullname).unwrap(); // TODO: !!!
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

        // we would be browsing only if we are a manager
        if self.is_manager {
            self.mdns.stop_browse(Self::MDNS_SERVICE_TYPE).unwrap(); // TODO: !!!
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
        self.register().await?;
        Ok(())
    }

    fn handle_service_resolved(&mut self, info: ServiceInfo) {
        if info.get_fullname().ends_with(Self::MDNS_SERVICE_TYPE) {
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

    #[inline]
    async fn handle_connection(&mut self, connection: TcpStream, socket: SocketAddr) {
        log::info!("Accepted connection from {}", socket);
        // TODO: exchange peer info here, so that we can record the fullname
    }
}
