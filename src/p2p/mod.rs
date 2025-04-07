use debug_print::debug_eprintln;
use futures::StreamExt;
use libp2p::core::transport::ListenerId;
use libp2p::{gossipsub, identity::Keypair, mdns, noise, tcp, yamux};
use libp2p::{swarm::SwarmEvent, Multiaddr};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

mod behaviour;
use behaviour::*;

mod gossip;

pub mod external;

pub struct DllmP2p {
    swarm: libp2p::Swarm<DLLMBehaviour>,
    cancellation: CancellationToken,
    message_tx: mpsc::UnboundedSender<gossipsub::Message>,
    message_rx: mpsc::UnboundedReceiver<gossipsub::Message>,
    listener_id: Option<ListenerId>,
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

        let (tx, rx) = mpsc::unbounded_channel::<gossipsub::Message>();

        Ok(Self {
            swarm,
            cancellation,
            message_tx: tx,
            message_rx: rx,
            listener_id: None,
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
    async fn shutdown(&mut self) {
        debug_eprintln!("Shutting down DLLMP2P");
        self.unsubscribe(Self::DLLM_TOPIC);
        debug_eprintln!("Shutting down channel");
        self.message_rx.close();
        while !self.message_rx.recv().await.is_none() { /* consume the channel */ }
        debug_eprintln!("Done");

        self.swarm.remove_listener(self.listener_id.unwrap());
    }

    #[inline]
    pub fn listen_on(&mut self, addr: Option<Multiaddr>) {
        const DEFAULT_ADDR: &str = "/ip4/0.0.0.0/tcp/0";
        let addr = addr.unwrap_or_else(|| DEFAULT_ADDR.parse().unwrap());

        self.listener_id = Some(self.swarm.listen_on(addr).expect("TODO: listen_on"));
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

        debug_eprintln!("Peer ID: {}", self.swarm.local_peer_id());
        loop {
            tokio::select! {
                _ = self.cancellation.cancelled() => {
                    self.shutdown().await;
                    return;
                },
                event = self.swarm.select_next_some() => self.handle_event(event).await,
            }
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
            SwarmEvent::Behaviour(DLLMBehaviourEvent::Gossipsub(event)) => {
                self.handle_gossipsub_event(event);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                debug_eprintln!("Local node is listening on {address}");
            }
            event => {
                log::debug!("Unhandled event: {event:?}");
            }
        }
    }
}
