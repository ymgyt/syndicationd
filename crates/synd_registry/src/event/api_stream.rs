use std::fmt;

use tokio::sync::broadcast;

use crate::{SubscriberId, event::ApiEvent};

#[derive(Clone)]
pub struct ApiEventPublisher {
    sender: broadcast::Sender<ApiEvent>,
}

pub struct ApiEventSubscriber {
    subscriber_id: SubscriberId,
    receiver: broadcast::Receiver<ApiEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiEventRecvError {
    Closed,
    Lagged(u64),
}

impl ApiEventPublisher {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self, subscriber_id: SubscriberId) -> ApiEventSubscriber {
        ApiEventSubscriber {
            subscriber_id,
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, event: ApiEvent) -> usize {
        self.sender.send(event).unwrap_or_default()
    }
}

impl Default for ApiEventPublisher {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl fmt::Debug for ApiEventPublisher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiEventPublisher").finish_non_exhaustive()
    }
}

impl ApiEventSubscriber {
    pub async fn recv(&mut self) -> Result<ApiEvent, ApiEventRecvError> {
        loop {
            let event = self.receiver.recv().await.map_err(|err| match err {
                broadcast::error::RecvError::Closed => ApiEventRecvError::Closed,
                broadcast::error::RecvError::Lagged(skipped) => ApiEventRecvError::Lagged(skipped),
            })?;
            if event_subscriber_id(&event) == &self.subscriber_id {
                return Ok(event);
            }
        }
    }
}

fn event_subscriber_id(event: &ApiEvent) -> &SubscriberId {
    match event {
        ApiEvent::FeedSubscribed(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedSubscribeRejected(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedSubscriptionChanged(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedUnsubscribed(event) => &event.subscription.subscriber_id,
        ApiEvent::FeedUnsubscribeRejected(event) => &event.subscription.subscriber_id,
    }
}
