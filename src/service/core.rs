use eyre::Context;
use mdns_sd::ServiceDaemon;
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

use super::ServiceProperties;

/// Listen on all interfaces on a random port.
const LISTEN_ADDR: &str = "0.0.0.0:0";

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
    /// A mapping of services from their `fullname` to their established connections.
    pub(crate) peer_conns: HashMap<String, TcpStream>,
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
}

impl DnetService {
    pub async fn new(cancellation: CancellationToken) -> eyre::Result<Self> {
        Ok(Self {
            cancellation,
            // TODO: do this elsewhere?
            listener: TcpListener::bind(LISTEN_ADDR)
                .await
                .wrap_err("failed to bind to TCP listener")?,
            sysinfo: sysinfo::System::new_all(),
            peer_conns: HashMap::new(),
            peer_props: HashMap::new(),
            properties: Default::default(),
            mdns: ServiceDaemon::new().wrap_err("failed to create mDNS service daemon")?,
        })
    }

    /// Returns the local port that the service is listening on.
    pub fn get_port(&self) -> eyre::Result<u16> {
        self.listener
            .local_addr()
            .map(|a| a.port())
            .wrap_err("could not get local address")
    }

    pub async fn start(&mut self) -> eyre::Result<()> {
        // create an interval to refresh properties
        const PROPERTY_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
        let mut property_refresh_interval = tokio::time::interval(PROPERTY_REFRESH_INTERVAL);
        property_refresh_interval.tick().await; // wait for the first tick

        loop {
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
              _ = property_refresh_interval.tick() => self.handle_property_refresh().await,
              _ = self.cancellation.cancelled() => break,
            }
        }

        Ok(())
    }

    /// Stops the service gracefully.
    async fn stop(&mut self) {
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
    async fn handle_property_refresh(&mut self) {
        self.sysinfo.refresh_all();
        self.properties.refresh_sysinfo(&self.sysinfo);
    }

    #[inline]
    async fn handle_connection(&mut self, connection: TcpStream, socket: SocketAddr) {
        log::info!("Accepted connection from {}", socket);
        // TODO: exchange peer info here, so that we can record the fullname
    }
}
