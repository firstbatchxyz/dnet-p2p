use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, identity::Keypair, mdns};
use std::{
    collections::hash_map,
    hash::{Hash, Hasher},
};
use tokio::{io, time::Duration};

#[derive(NetworkBehaviour)]
pub(crate) struct DLLMBehaviour {
    pub(crate) gossipsub: gossipsub::Behaviour,
    pub(crate) mdns: mdns::tokio::Behaviour,
}

impl DLLMBehaviour {
    #[inline]
    pub fn new(key: &Keypair) -> Self {
        Self {
            gossipsub: create_gossipsub_behaviour(key).expect("TODO: gossipsub"),
            mdns: create_mdns_behaviour(key).expect("TODO: mdns"),
        }
    }
}

#[inline]
fn create_gossipsub_behaviour(
    keypair: &Keypair,
) -> Result<gossipsub::Behaviour, Box<dyn std::error::Error>> {
    use gossipsub::{Behaviour, ConfigBuilder, ValidationMode};
    use gossipsub::{Message, MessageAuthenticity, MessageId};

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

    let config = Config::default();
    // NOTE: we can set a custom TTL here if we want to,
    // but a low TTL causes nodes to be expired inadvertently!

    let mdns = Behaviour::new(config, keypair.public().to_peer_id())?;
    Ok(mdns)
}
