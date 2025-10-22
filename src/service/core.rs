use eyre::Context;
use std::collections::HashMap;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use super::{
    udp::{spawn_udp_task, UdpCommand, UdpEvent, UdpHandle, UdpMessage},
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
    /// A mapping of services from their `instance` name to their last-seen properties.
    pub peer_props: HashMap<String, DnetServiceProperties>,
    /// A shared service properties object.
    ///
    /// This is published via UDP to all other services.
    pub(crate) properties: DnetServiceProperties,
    /// UDP worker handle and channels.
    pub(crate) udp_handle: Option<UdpHandle>,
    /// Whether this service is a manager or not.
    pub(crate) is_manager: bool,
    /// Whether this service is passive (monitors only, doesn't register).
    pub(crate) is_passive: bool,
}

impl DnetService {
    pub fn new(
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

        Ok(Self {
            cancellation,
            peer_props: HashMap::new(),
            properties,
            udp_handle: None,
            is_manager,
            is_passive,
        })
    }
    /// Starts the service with UDP discovery worker.
    pub async fn start(&mut self) -> eyre::Result<()> {
        // spawn UDP worker task
        let handle = spawn_udp_task(self.properties.clone(), self.is_passive)
            .wrap_err("failed to spawn UDP worker task")?;
        log::info!(
            "Started UDP worker for instance {}",
            self.properties.instance
        );
        self.udp_handle = Some(handle);

        loop {
            // await either an event or cancellation
            let evt_opt: Option<UdpEvent> = tokio::select! {
                evt = self
                        .udp_handle
                        .as_mut()
                        .expect("udp_handle set")
                        .evt_rx
                        .recv() => evt,
                _ = self.cancellation.cancelled() => break,
            };

            if let Some(evt) = evt_opt {
                match evt {
                    UdpEvent::Message(msg, sender_addr) => {
                        self.handle_udp_message(msg, sender_addr);
                    }
                    UdpEvent::PeersTimedOut(timed_out_instances) => {
                        for instance in timed_out_instances {
                            self.peer_props.remove(&instance);
                            log::info!("Removed stale peer: {}", instance);
                        }
                    }
                }
            } else {
                // channel closed; break out to stop
                log::warn!("UDP event channel closed");
                break;
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
        if let Some(handle) = self.udp_handle.take() {
            // broadcast removal notification if not passive
            if !self.is_passive {
                log::info!(
                    "Broadcasting REMOVE notification for instance {}",
                    self.properties.instance
                );
                if let Err(e) = handle
                    .cmd_tx
                    .send(UdpCommand::BroadcastRemove(
                        self.properties.instance.clone(),
                    ))
                    .await
                {
                    log::error!("Failed to request removal broadcast: {e}");
                } else {
                    log::info!("Requested REMOVE notification successfully");
                }

                // delay to ensure the removal message is sent before shutdown
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }

            // request shutdown
            if let Err(e) = handle.cmd_tx.send(UdpCommand::Shutdown).await {
                log::warn!("UDP worker command channel closed before shutdown: {e}");
            }

            // wait for worker task to complete
            match handle.task.await {
                Ok(Ok(())) => {
                    log::info!("UDP worker shut down cleanly");
                }
                Ok(Err(e)) => {
                    log::error!("UDP worker returned error: {e}");
                }
                Err(e) => {
                    log::error!("UDP worker join error: {e}");
                }
            }
        }
    }

    /// Sets the busy status of the service.
    pub async fn set_is_busy(&mut self, is_busy: bool) {
        // handle no-ops & passive
        if self.properties.is_busy == is_busy || self.is_passive {
            return;
        }
        self.properties.is_busy = is_busy;

        // send updated properties to UDP worker and request immediate broadcast
        if let Some(handle) = self.udp_handle.as_ref() {
            if let Err(e) = handle
                .cmd_tx
                .send(UdpCommand::UpdateProperties(self.properties.clone()))
                .await
            {
                log::warn!("Failed to send UpdateProperties to UDP worker: {e}");
            }
            if let Err(e) = handle.cmd_tx.send(UdpCommand::BroadcastNow).await {
                log::debug!("Failed to request immediate broadcast: {e}");
            }
        }

        log::debug!("Set is_busy to {is_busy}");
    }
}
