use debug_print::debug_eprintln;
use libp2p::gossipsub::{Event, IdentTopic, MessageId, PublishError, SubscriptionError};

impl super::DllmP2p {
    /// Subscribes to the given topic.
    #[inline]
    pub(super) fn subscribe(
        &mut self,
        topic: impl Into<String>,
    ) -> Result<bool, SubscriptionError> {
        let topic = topic.into();
        log::debug!("Subscribed from {}", topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&IdentTopic::new(topic))
    }

    /// Unsubscribes from the given topic.
    #[inline]
    pub(super) fn unsubscribe(&mut self, topic: impl Into<String>) -> bool {
        let topic = topic.into();
        log::debug!("Unsubscribing from {}", topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .unsubscribe(&IdentTopic::new(topic))
    }

    /// Publishes data to the given topic.
    pub(super) fn publish(
        &mut self,
        topic: impl Into<String>,
        data: impl Into<Vec<u8>>,
    ) -> Result<MessageId, PublishError> {
        let topic = topic.into();
        log::debug!("Publishing data to {}", topic);
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(IdentTopic::new(topic), data)
    }

    #[inline]
    pub fn handle_gossipsub_event(&mut self, event: Event) {
        match event {
            Event::Message {
                propagation_source,
                message,
                ..
            } => {
                log::debug!("Received message: {:?}", message);
                debug_eprintln!(
                    "Got message ({} bytes) from {propagation_source}",
                    message.data.len()
                );
                if let Err(e) = self.message_tx.send(message) {
                    debug_eprintln!("Failed to send message: {e}");
                }
            }
            _ => {}
        }
    }
}
