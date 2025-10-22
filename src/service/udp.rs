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
    /// Broadcast interval duration
    pub broadcast_interval: Duration,
    /// Cleanup interval duration
    pub cleanup_interval: Duration,
    /// Last seen time for each peer (keyed by their instance name)
    peer_last_seen: HashMap<String, Instant>,
    /// Timeout duration for considering a peer as offline
    peer_timeout: Duration,
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

        let broadcast_interval_secs = env::var("DNET_P2P_UDP_BROADCAST_INTERVAL")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_BROADCAST_INTERVAL);
        let broadcast_interval = Duration::from_secs(broadcast_interval_secs);

        log::info!(
            "UDP discovery initialized on port {} (broadcast to {})",
            port,
            broadcast_addr
        );

        Ok(Self {
            socket,
            port,
            broadcast_addr,
            broadcast_interval,
            peer_last_seen: HashMap::new(),
            peer_timeout: Duration::from_secs(peer_timeout_secs),
            cleanup_interval: Duration::from_secs(2),
        })
    }

    /// Broadcasts own service properties via UDP
    pub async fn broadcast_properties(
        &self,
        properties: &DnetServiceProperties,
    ) -> eyre::Result<()> {
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
}
