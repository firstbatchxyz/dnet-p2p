use eyre::Context;
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;

use super::DnetServiceProperties;

/// Message received from UDP discovery
#[derive(Debug)]
pub enum UdpMessage {
    /// Update message: peer is announcing their properties
    Update(DnetServiceProperties),
    /// Remove message: peer is gracefully shutting down
    Remove(String), // instance name
}

impl UdpMessage {
    pub const MSG_TYPE_UPDATE: u8 = b'U';
    pub const MSG_TYPE_REMOVE: u8 = b'R';
}

/// UDP discovery configuration and state
pub struct UdpDiscovery {
    /// The UDP socket for sending and receiving broadcasts
    socket: UdpSocket,
    /// Port to broadcast/listen on
    pub port: u16,
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
    /// Magic word to identify dnet UDP packets (4 bytes: "dnet")
    const MAGIC_WORD: &'static [u8; 4] = b"dnet";

    /// Default UDP port for dnet discovery
    pub const DEFAULT_PORT: u16 = 0;

    /// Default broadcast interval in seconds
    pub const DEFAULT_BROADCAST_INTERVAL: u64 = 3;

    /// Default peer timeout in seconds
    pub const DEFAULT_PEER_TIMEOUT: u64 = 10;

    /// Creates a new UDP discovery instance
    pub async fn new() -> eyre::Result<Self> {
        // load configuration from environment variables
        let mut port = env::var("DNET_P2P_UDP_PORT")
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

        // get the actual port assigned by the OS (in case port 0 was used)
        port = socket
            .local_addr()
            .wrap_err("failed to get socket local address")?
            .port();

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

    /// Broadcasts own service properties via UDP (Update message)
    pub async fn broadcast_properties(
        &self,
        properties: &DnetServiceProperties,
    ) -> eyre::Result<()> {
        // serialize properties to JSON
        let json = serde_json::to_string(properties)
            .wrap_err("failed to serialize service properties to JSON")?;

        // build packet: magic word + 'U' + JSON payload
        let mut payload = Vec::with_capacity(Self::MAGIC_WORD.len() + 1 + json.len());
        payload.extend_from_slice(Self::MAGIC_WORD);
        payload.push(UdpMessage::MSG_TYPE_UPDATE);
        payload.extend_from_slice(json.as_bytes());

        // broadcast the payload
        let bytes_sent = self
            .socket
            .send_to(&payload, self.broadcast_addr)
            .await
            .wrap_err("failed to send UDP broadcast")?;

        log::debug!(
            "Broadcasted UPDATE {} bytes (magic + type + {} JSON bytes) to {} (instance: {})",
            bytes_sent,
            json.len(),
            self.broadcast_addr,
            properties.instance
        );

        Ok(())
    }

    /// Broadcasts a removal notification (Remove message)
    ///
    /// Signals graceful shutdown by sending the instance name for peers to remove
    pub async fn broadcast_remove(&self, instance: &str) -> eyre::Result<()> {
        // build packet: magic word + 'R' + instance name
        let mut payload =
            Vec::with_capacity(Self::MAGIC_WORD.len() + 1 + instance.as_bytes().len());
        payload.extend_from_slice(Self::MAGIC_WORD);
        payload.push(UdpMessage::MSG_TYPE_REMOVE);
        payload.extend_from_slice(instance.as_bytes());

        // broadcast the removal notification
        let bytes_sent = self
            .socket
            .send_to(&payload, self.broadcast_addr)
            .await
            .wrap_err("failed to send UDP removal broadcast")?;

        log::info!(
            "Broadcasted REMOVE {} bytes to {} (instance: {})",
            bytes_sent,
            self.broadcast_addr,
            instance
        );

        Ok(())
    }

    /// Receives and parses peer messages from UDP
    ///
    /// Returns `Some((message, sender_addr))` if a valid message was received,
    /// or `None` if the operation would block or no data is available.
    pub async fn receive_announcement(&mut self) -> eyre::Result<Option<(UdpMessage, SocketAddr)>> {
        let mut buf = vec![0u8; 65535]; // max UDP packet size

        // try to receive data (non-blocking via tokio)
        match self.socket.recv_from(&mut buf).await {
            Ok((len, sender_addr)) => {
                log::debug!("Received {} bytes from {}", len, sender_addr);

                // check if packet is large enough to contain magic word + message type
                if len < Self::MAGIC_WORD.len() + 1 {
                    log::warn!(
                        "Ignoring packet from {} (too small: {} bytes)",
                        sender_addr,
                        len
                    );
                    return Ok(None);
                }

                // validate magic word
                if &buf[..Self::MAGIC_WORD.len()] != Self::MAGIC_WORD {
                    log::warn!("Ignoring packet from {} (invalid magic word)", sender_addr);
                    return Ok(None);
                }

                // extract message type
                let msg_type = buf[Self::MAGIC_WORD.len()];
                let payload = &buf[Self::MAGIC_WORD.len() + 1..len];

                match msg_type {
                    UdpMessage::MSG_TYPE_UPDATE => {
                        // parse JSON payload
                        let json_str = std::str::from_utf8(payload)
                            .wrap_err("failed to parse UDP payload as UTF-8")?;

                        let properties: DnetServiceProperties = serde_json::from_str(json_str)
                            .wrap_err("failed to deserialize service properties from JSON")?;

                        // update last seen time for this peer
                        self.peer_last_seen
                            .insert(properties.instance.clone(), Instant::now());

                        log::info!(
                            "Received UPDATE from {} at {}",
                            properties.instance,
                            sender_addr,
                        );

                        Ok(Some((UdpMessage::Update(properties), sender_addr)))
                    }
                    UdpMessage::MSG_TYPE_REMOVE => {
                        // parse instance name from payload
                        let instance = std::str::from_utf8(payload)
                            .wrap_err("failed to parse instance name as UTF-8")?
                            .to_string();

                        log::info!(
                            "Received REMOVE from instance '{}' at {}",
                            instance,
                            sender_addr
                        );

                        Ok(Some((UdpMessage::Remove(instance), sender_addr)))
                    }
                    _ => {
                        log::trace!(
                            "Ignoring packet from {} (unknown message type: {})",
                            sender_addr,
                            msg_type as char
                        );
                        Ok(None)
                    }
                }
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
