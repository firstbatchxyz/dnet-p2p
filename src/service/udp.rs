use eyre::Context;
// use socket2::Domain;
use socket_pktinfo::PktInfoUdpSocket;
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
// use tokio_util::sync::CancellationToken;

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

/// Commands sent from the core service to the UDP worker task
#[derive(Debug)]
pub(crate) enum UdpCommand {
    /// Immediately broadcast current properties
    BroadcastNow,
    /// Replace current properties used by the worker
    UpdateProperties(DnetServiceProperties),
    /// Broadcast a graceful removal message for the given instance
    BroadcastRemove(String),
    /// Request the worker to shutdown
    Shutdown,
}

/// Events sent from the UDP worker to the core service
#[derive(Debug)]
pub(crate) enum UdpEvent {
    /// A parsed UDP message from a peer
    Message(UdpMessage, SocketAddr),
    /// List of peers that timed out
    PeersTimedOut(Vec<String>),
}

/// Handle for interacting with the UDP worker task
pub(crate) struct UdpHandle {
    pub(crate) cmd_tx: mpsc::Sender<UdpCommand>,
    pub(crate) evt_rx: mpsc::Receiver<UdpEvent>,
    pub(crate) task: JoinHandle<eyre::Result<()>>,
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
    /// Socket buffer (created once to avoid reallocations)
    socket_buf: Vec<u8>,
}

impl UdpDiscovery {
    /// Magic word to identify dnet UDP packets (4 bytes: "dnet")
    const MAGIC_WORD: &'static [u8; 4] = b"dnet";

    /// Default UDP port for dnet discovery
    /// All instances must use the same port for broadcast discovery to work
    pub const DEFAULT_PORT: u16 = 37021;

    /// Default broadcast interval in seconds
    pub const DEFAULT_BROADCAST_INTERVAL: u64 = 3;

    /// Default cleanup interval in seconds
    pub const DEFAULT_CLEANUP_INTERVAL: u64 = 5;

    /// Default peer timeout in seconds
    ///
    /// Peers not seen within this duration are considered offline, and are dropped.
    pub const DEFAULT_PEER_TIMEOUT: u64 = Self::DEFAULT_BROADCAST_INTERVAL * 3;

    /// Creates a new UDP discovery instance
    pub fn new() -> eyre::Result<Self> {
        // load configuration from environment variables
        let port = Self::DEFAULT_PORT;

        let peer_timeout_secs = env::var("DNET_P2P_UDP_PEER_TIMEOUT")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_PEER_TIMEOUT);

        // create UDP socket with SO_REUSEADDR to allow multiple bindings
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let pkt_socket =
            PktInfoUdpSocket::new(socket2::Domain::IPV4).wrap_err("failed to create UDP socket")?;

        pkt_socket
            .set_reuse_address(true)
            .wrap_err("failed to enable address reuse")?;
        #[cfg(unix)] // unix only
        pkt_socket
            .set_reuse_port(true)
            .wrap_err("failed to enable port reuse")?;

        // FIXME: is this needed? we will run in a separate thread anyways
        pkt_socket
            .set_nonblocking(true)
            .wrap_err("failed to enable non-blocking mode")?;

        // bind
        pkt_socket
            .bind(&bind_addr.into())
            .wrap_err("failed to bind UDP socket")?;

        // convert socket2 -> std -> tokio
        let socket = UdpSocket::from_std(pkt_socket.try_clone_std().unwrap())
            .wrap_err("failed to convert to tokio socket")?;
        socket
            .set_broadcast(true)
            .wrap_err("failed to enable broadcast")?;

        let broadcast_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), port);

        let broadcast_interval_secs = env::var("DNET_P2P_UDP_BROADCAST_INTERVAL")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_BROADCAST_INTERVAL);

        let cleanup_interval_secs = env::var("DNET_P2P_UDP_CLEANUP_INTERVAL")
            .ok()
            .and_then(|val| val.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_CLEANUP_INTERVAL);

        log::info!(
            "UDP discovery initialized on port {} (broadcast to {})",
            port,
            broadcast_addr
        );

        Ok(Self {
            socket,
            socket_buf: vec![0u8; 65535], // max UDP packet size
            port,
            broadcast_addr,
            peer_last_seen: HashMap::new(),
            broadcast_interval: Duration::from_secs(broadcast_interval_secs),
            cleanup_interval: Duration::from_secs(cleanup_interval_secs),
            peer_timeout: Duration::from_secs(peer_timeout_secs),
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
        // try to receive data (non-blocking via tokio)
        match self.socket.recv_from(&mut self.socket_buf).await {
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
                if &self.socket_buf[..Self::MAGIC_WORD.len()] != Self::MAGIC_WORD {
                    log::warn!("Ignoring packet from {} (invalid magic word)", sender_addr);
                    return Ok(None);
                }

                // extract message type
                let msg_type = self.socket_buf[Self::MAGIC_WORD.len()];
                let payload = &self.socket_buf[Self::MAGIC_WORD.len() + 1..len];

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
                            "Received REMOVE from instance {} (from: {})",
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
                    log::error!(
                        "UDP recv_from error: kind={:?}, os_error={:?}, message={}",
                        e.kind(),
                        e.raw_os_error(),
                        e
                    );
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

/// Spawns the UDP discovery worker task and returns a handle for interaction
pub(crate) fn spawn_udp_task(
    initial_properties: DnetServiceProperties,
    is_passive: bool,
) -> eyre::Result<UdpHandle> {
    // channels
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<UdpCommand>(32);
    let (evt_tx, evt_rx) = mpsc::channel::<UdpEvent>(64);

    // create discovery synchronously before spawning so errors bubble up
    let mut discovery = UdpDiscovery::new().wrap_err("failed to create UDP discovery")?;

    let task: JoinHandle<eyre::Result<()>> = tokio::spawn(async move {
        let mut properties = initial_properties;

        // timers
        let mut broadcast_interval = tokio::time::interval(discovery.broadcast_interval);
        broadcast_interval.tick().await;
        let mut cleanup_interval = tokio::time::interval(discovery.cleanup_interval);
        cleanup_interval.tick().await;

        log::info!(
            "UDP worker started on port {} for instance {}",
            discovery.port,
            properties.instance
        );

        loop {
            tokio::select! {
                // commands from core
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(UdpCommand::BroadcastNow) => {
                            if !is_passive {
                                if let Err(e) = discovery.broadcast_properties(&properties).await {
                                    log::error!("UDP worker: broadcast now failed: {e}");
                                }
                            }
                        }
                        Some(UdpCommand::UpdateProperties(p)) => {
                            properties = p;
                        }
                        Some(UdpCommand::BroadcastRemove(instance)) => {
                            if let Err(e) = discovery.broadcast_remove(&instance).await {
                                log::error!("UDP worker: broadcast remove failed: {e}");
                            }
                        }
                        Some(UdpCommand::Shutdown) => {
                            log::info!("UDP worker: received shutdown command");
                            break;
                        }
                        None => {
                            // command channel closed by core; exit loop
                            log::warn!("UDP worker: command channel closed, shutting down");
                            break;
                        }
                    }
                }

                // periodic broadcast
                _ = broadcast_interval.tick() => {
                    if !is_passive {
                        if let Err(e) = discovery.broadcast_properties(&properties).await {
                            log::error!("UDP worker: periodic broadcast failed: {e}");
                        }
                    }
                }

                // periodic cleanup
                _ = cleanup_interval.tick() => {
                    let timed_out = discovery.cleanup_stale_peers();
                    if !timed_out.is_empty() {
                        if let Err(e) = evt_tx.send(UdpEvent::PeersTimedOut(timed_out)).await {
                            log::debug!("UDP worker: failed to send PeersTimedOut event: {e}");
                        }
                    }
                }

                // socket receive
                recv = discovery.receive_announcement() => {
                    match recv {
                        Ok(Some((msg, addr))) => {
                            if let Err(e) = evt_tx.send(UdpEvent::Message(msg, addr)).await {
                                log::debug!("UDP worker: failed to send Message event: {e}");
                            }
                        }
                        Ok(None) => { /* no packet */ }
                        Err(e) => {
                            log::error!("UDP worker: receive error: {e}");

                            // brief backoff
                            tokio::time::sleep(Duration::from_millis(250)).await;
                        }
                    }
                }
            }
        }

        log::info!("UDP worker: exiting");
        Ok(())
    });

    Ok(UdpHandle {
        cmd_tx,
        evt_rx,
        task,
    })
}
