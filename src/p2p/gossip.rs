use libp2p::gossipsub::{IdentTopic, SubscriptionError};

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
}
