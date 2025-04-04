use debug_print::debug_eprintln;
use futures::StreamExt;
use libp2p::{gossipsub, identity::Keypair, mdns, noise, tcp, yamux};
use libp2p::{swarm::SwarmEvent, Multiaddr};

use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

mod behaviour;
use behaviour::*;

mod gossip;

pub mod external;

pub struct DllmP2p {
    swarm: libp2p::Swarm<DLLMBehaviour>,
    cancellation: CancellationToken,
}

impl DllmP2p {
    /// The default topic to subscribe to.
    pub const DLLM_TOPIC: &'static str = "dllm";

    pub fn new(
        keypair: Keypair,
        cancellation: CancellationToken,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| Ok(DLLMBehaviour::new(key)))?
            .build();

        Ok(Self {
            swarm,
            cancellation,
        })
    }

    /// Triggers cancellation.
    #[inline]
    fn stop(&mut self) {
        if !self.cancellation.is_cancelled() {
            self.cancellation.cancel();
        }
    }

    /// Shuts down the application.
    #[inline]
    fn shutdown(&mut self) {
        debug_eprintln!("Shutting down DLLMP2P");
        self.unsubscribe(Self::DLLM_TOPIC);
    }

    #[inline]
    pub fn listen_on(&mut self, addr: Option<Multiaddr>) {
        const DEFAULT_ADDR: &str = "/ip4/0.0.0.0/tcp/0";

        self.swarm
            .listen_on(addr.unwrap_or_else(|| DEFAULT_ADDR.parse().unwrap()))
            .expect("TODO: listen_on");
    }

    /// Waits for swarm events and Node commands at the same time.
    ///
    /// To terminate, the command channel must be closed.
    ///
    /// Can be inlined because its a main loop.
    #[inline]
    pub async fn run_daemon(&mut self, addr: Option<Multiaddr>) {
        self.subscribe(Self::DLLM_TOPIC).unwrap();
        self.listen_on(addr);

        debug_eprintln!("Peer id: {}", self.swarm.local_peer_id());
        loop {
            tokio::select! {
                _ = self.cancellation.cancelled() => {
                    self.shutdown();
                    break;
                },
                event = self.swarm.select_next_some() => self.handle_event(event).await,
            }
        }
    }

    pub async fn run_topo(&mut self, duration: Duration) {
        self.subscribe(Self::DLLM_TOPIC).expect("TODO: !!!");
        self.listen_on(None);

        // collect events for the given duration
        let mut ticker = tokio::time::interval(duration);
        ticker.tick().await;
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                _ = self.cancellation.cancelled() => {
                    self.shutdown();
                    return;
                },
                _ = ticker.tick() => break,
            }
        }

        // print discovered nodes
        for peer in self.swarm.behaviour().mdns.discovered_nodes() {
            log::info!("{:?}", peer);
        }
    }

    /// Handles the returned [`libp2p::swarm::SwarmEvent`].
    async fn handle_event(&mut self, event: SwarmEvent<DLLMBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(DLLMBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, _multiaddr) in list {
                    debug_eprintln!("mDNS discovered a new peer: {peer_id}");
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                }
            }
            SwarmEvent::Behaviour(DLLMBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _multiaddr) in list {
                    debug_eprintln!("mDNS discover peer has expired: {peer_id}");
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
            })) => {
                debug_eprintln!(
                    "Got message ({id}) from {peer_id}\n{}",
                    String::from_utf8_lossy(&message.data)
                );
                // TODO: !!!
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                debug_eprintln!("Local node is listening on {address}");
            }
            event => {
                log::debug!("SwarmEvent: {event:?}")
            }
        }
    }
}
