use futures::StreamExt;
use libp2p::{gossipsub, identity::Keypair, mdns, noise, tcp, yamux};
use libp2p::{
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr,
};
use std::{
    collections::hash_map,
    hash::{Hash, Hasher},
};
use tokio::time::Duration;

use tokio::io;
use tokio_util::sync::CancellationToken;

#[derive(NetworkBehaviour)]
pub struct DLLMBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct DLLMP2P {
    swarm: libp2p::Swarm<DLLMBehaviour>,
}

const DLLM_TOPIC: &str = "dllm";

impl DLLMP2P {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                Ok(DLLMBehaviour {
                    gossipsub: create_gossipsub_behaviour(key).expect("TODO: gossipsub"),
                    mdns: create_mdns_behaviour(key).expect("TODO: mdns"),
                })
            })?
            .build();

        Ok(Self { swarm })
    }

    /// Subscribes to the given topic.
    #[inline]
    fn subscribe(
        &mut self,
        topic: impl Into<String>,
    ) -> Result<bool, gossipsub::SubscriptionError> {
        let topic = topic.into();
        log::debug!("Subscribed from {}", topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&gossipsub::IdentTopic::new(topic))
    }

    /// Unsubscribes from the given topic.
    #[inline]
    fn unsubscribe(&mut self, topic: impl Into<String>) -> bool {
        let topic = topic.into();
        log::debug!("Unsubscribing from {}", topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .unsubscribe(&gossipsub::IdentTopic::new(topic))
    }

    /// Waits for swarm events and Node commands at the same time.
    ///
    /// To terminate, the command channel must be closed.
    pub async fn run(&mut self, cancellation: CancellationToken, addr: Option<Multiaddr>) {
        self.subscribe(DLLM_TOPIC).unwrap();
        self.listen_on(addr);

        log::info!("Peer id: {}", self.swarm.local_peer_id());
        loop {
            tokio::select! {
                _ = cancellation.cancelled() => {
                    self.shutdown();
                    break;
                },
                event = self.swarm.select_next_some() => self.handle_event(event).await,
            }
        }
    }

    fn shutdown(&mut self) {
        log::info!("Terminating the application...");
        self.unsubscribe(DLLM_TOPIC);
    }

    pub async fn get_topology(&mut self, cancellation: CancellationToken, duration: Duration) {
        self.subscribe(DLLM_TOPIC).unwrap();
        self.listen_on(None);

        // collect events for the given duration
        let mut ticker = tokio::time::interval(duration);
        ticker.tick().await;
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                _ = cancellation.cancelled() => {
                    self.shutdown();
                    return;
                },
                _ = ticker.tick() => {
                    break;
                }
            }
        }

        // print discovered nodes
        for peer in self.swarm.behaviour().mdns.discovered_nodes() {
            log::info!("{:?}", peer);
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<DLLMBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(DLLMBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, _multiaddr) in list {
                    log::info!("mDNS discovered a new peer: {peer_id}");
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                }
            }
            SwarmEvent::Behaviour(DLLMBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _multiaddr) in list {
                    log::info!("mDNS discover peer has expired: {peer_id}");
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            }
            SwarmEvent::Behaviour(DLLMBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: peer_id,
                message_id: id,
                message,
            })) => log::info!(
                "Got message ({id}) from {peer_id}\n{}",
                String::from_utf8_lossy(&message.data),
            ),
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Local node is listening on {address}");
            }
            _ => {}
        }
    }

    #[inline]
    pub fn listen_on(&mut self, addr: Option<Multiaddr>) {
        const DEFAULT_ADDR: &str = "/ip4/0.0.0.0/tcp/0";

        self.swarm
            .listen_on(addr.unwrap_or_else(|| DEFAULT_ADDR.parse().unwrap()))
            .expect("TODO: listen_on");
    }
}

#[inline]
fn create_gossipsub_behaviour(
    keypair: &Keypair,
) -> Result<gossipsub::Behaviour, Box<dyn std::error::Error>> {
    use gossipsub::{Behaviour, ConfigBuilder, MessageAuthenticity, ValidationMode};
    use gossipsub::{Message, MessageId};

    // message id's are simply hashes of the message data, via SipHash13
    let message_id_fn = |message: &Message| {
        let mut hasher = hash_map::DefaultHasher::new();
        message.data.hash(&mut hasher);
        MessageId::from(hasher.finish().to_be_bytes())
    };

    /// Time between each GossipSub heartbeat.
    const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

    // permissive mode with author peer ids only, good for constrained devices to avoid signatures
    let gossipsub_config = ConfigBuilder::default()
        .heartbeat_interval(HEARTBEAT_INTERVAL)
        .validation_mode(ValidationMode::Permissive)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

    let gossipsub = Behaviour::new(
        MessageAuthenticity::Author(keypair.public().to_peer_id()),
        gossipsub_config,
    )?;

    Ok(gossipsub)
}

#[inline]
fn create_mdns_behaviour(
    keypair: &Keypair,
) -> Result<mdns::tokio::Behaviour, Box<dyn std::error::Error>> {
    use mdns::tokio::Behaviour;
    use mdns::Config;

    let mdns = Behaviour::new(Config::default(), keypair.public().to_peer_id())?;
    Ok(mdns)
}
