use eyre::Context;
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;

use super::DnetServiceProperties;

/// UDP discovery configuration and state
pub struct UdpDiscovery {
    /// The UDP socket for sending and receiving broadcasts
    socket: UdpSocket,
    /// Port to broadcast/listen on
    port: u16,
    /// Broadcast address
    broadcast_addr: SocketAddr,
    /// Last seen time for each peer (keyed by their instance name)
    peer_last_seen: HashMap<String, Instant>,
    /// Timeout duration for considering a peer as offline
    peer_timeout: Duration,
    /// Whether UDP discovery is enabled
    enabled: bool,
}

impl UdpDiscovery {
    /// Default UDP port for dnet discovery
    pub const DEFAULT_PORT: u16 = 37020;

    /// Default broadcast interval in seconds
    pub const DEFAULT_BROADCAST_INTERVAL: u64 = 3;

    /// Default peer timeout in seconds
    pub const DEFAULT_PEER_TIMEOUT: u64 = 10;

    /// Creates a new UDP discovery instance
    pub async fn new() -> eyre::Result<Self> {
        // check if UDP discovery is enabled
        let enabled = env::var("DNET_P2P_UDP_ENABLED")
            .ok()
            .and_then(|val| val.parse::<bool>().ok())
            .unwrap_or(true);

        if !enabled {
            log::info!("UDP discovery is disabled");
            // return a dummy instance
            let socket = UdpSocket::bind("0.0.0.0:0")
                .await
                .wrap_err("failed to bind UDP socket")?;
            return Ok(Self {
                socket,
                port: 0,
                broadcast_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), 0),
                peer_last_seen: HashMap::new(),
                peer_timeout: Duration::from_secs(Self::DEFAULT_PEER_TIMEOUT),
                enabled: false,
            });
        }

        // load configuration from environment variables
        let port = env::var("DNET_P2P_UDP_PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .unwrap_or(Self::DEFAULT_PORT);

        let peer_timeout_secs = env::var("DNET_P2P_UDP_PEER_TIMEOUT")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_PEER_TIMEOUT);

        // create UDP socket
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let socket = UdpSocket::bind(bind_addr)
            .await
            .wrap_err("failed to bind UDP socket")?;

        // enable broadcast
        socket
            .set_broadcast(true)
            .wrap_err("failed to set broadcast on UDP socket")?;

        let broadcast_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), port);

        log::info!(
            "UDP discovery initialized on port {} (broadcast to {})",
            port,
            broadcast_addr
        );

        Ok(Self {
            socket,
            port,
            broadcast_addr,
            peer_last_seen: HashMap::new(),
            peer_timeout: Duration::from_secs(peer_timeout_secs),
            enabled,
        })
    }

    /// Returns whether UDP discovery is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the broadcast interval duration from environment or default
    pub fn get_broadcast_interval() -> Duration {
        let interval_secs = env::var("DNET_P2P_UDP_BROADCAST_INTERVAL")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_BROADCAST_INTERVAL);

        Duration::from_secs(interval_secs)
    }

    /// Broadcasts own service properties via UDP
    pub async fn broadcast_properties(
        &self,
        properties: &DnetServiceProperties,
    ) -> eyre::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // serialize properties to JSON
        let json = serde_json::to_string(properties)
            .wrap_err("failed to serialize service properties to JSON")?;

        // broadcast the JSON payload
        let bytes_sent = self
            .socket
            .send_to(json.as_bytes(), self.broadcast_addr)
            .await
            .wrap_err("failed to send UDP broadcast")?;

        log::debug!(
            "Broadcasted {} bytes to {} (instance: {})",
            bytes_sent,
            self.broadcast_addr,
            properties.instance
        );

        Ok(())
    }

    /// Receives and parses peer announcements from UDP
    ///
    /// Returns `Some((properties, sender_addr))` if a valid announcement was received,
    /// or `None` if the operation would block or no data is available.
    pub async fn receive_announcement(
        &mut self,
    ) -> eyre::Result<Option<(DnetServiceProperties, SocketAddr)>> {
        if !self.enabled {
            return Ok(None);
        }

        let mut buf = vec![0u8; 65535]; // max UDP packet size

        // try to receive data (non-blocking via tokio)
        match self.socket.recv_from(&mut buf).await {
            Ok((len, sender_addr)) => {
                log::debug!("Received {} bytes from {}", len, sender_addr);

                // parse the JSON payload
                let json_str = std::str::from_utf8(&buf[..len])
                    .wrap_err("failed to parse UDP payload as UTF-8")?;

                let properties: DnetServiceProperties = serde_json::from_str(json_str)
                    .wrap_err("failed to deserialize service properties from JSON")?;

                // update last seen time for this peer
                self.peer_last_seen
                    .insert(properties.instance.clone(), Instant::now());

                log::info!(
                    "Received announcement from {} at {} (manager: {}, busy: {})",
                    properties.instance,
                    sender_addr,
                    properties.is_manager,
                    properties.is_busy
                );

                Ok(Some((properties, sender_addr)))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    // no data available, not an error
                    Ok(None)
                } else {
                    Err(e).wrap_err("failed to receive UDP packet")
                }
            }
        }
    }

    /// Removes peers that haven't been seen within the timeout period
    ///
    /// Returns a list of instance names that timed out
    pub fn cleanup_stale_peers(&mut self) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }

        let now = Instant::now();
        let mut timed_out = Vec::new();

        // find peers that have timed out
        self.peer_last_seen.retain(|instance, last_seen| {
            let elapsed = now.duration_since(*last_seen);
            if elapsed > self.peer_timeout {
                log::warn!(
                    "Peer {} timed out (last seen {:.1}s ago)",
                    instance,
                    elapsed.as_secs_f64()
                );
                timed_out.push(instance.clone());
                false
            } else {
                true
            }
        });

        timed_out
    }

    /// Returns the peer timeout check interval (should be checked periodically)
    pub fn get_cleanup_interval() -> Duration {
        // check for stale peers every 2 seconds
        Duration::from_secs(2)
    }
}
