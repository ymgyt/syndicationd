use std::fmt;

use tokio::sync::broadcast;
use tracing::debug;

use crate::{
    SubscriberId,
    event::{
        ApiEvent, ApiEventKind, Event, EventInterests, Processor, ProcessorError, ProcessorId,
        ProcessorResult, Sink,
    },
};

/// Broadcasts committed API events to subscriber-scoped listeners.
#[derive(Clone)]
pub struct ApiEventPublisher {
    sender: broadcast::Sender<ApiEvent>,
}

/// Receives API events for one subscriber.
pub struct ApiEventSubscriber {
    subscriber_id: SubscriberId,
    receiver: broadcast::Receiver<ApiEvent>,
}

/// Error returned while receiving API events.
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

impl Processor for ApiEventPublisher {
    type Input = ApiEvent;

    fn id(&self) -> ProcessorId {
        ProcessorId::ApiEventPublisher
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            ApiEventKind::FeedSubscribed.into(),
            ApiEventKind::FeedSubscribeRejected.into(),
            ApiEventKind::FeedSubscriptionChanged.into(),
            ApiEventKind::FeedUnsubscribed.into(),
            ApiEventKind::FeedUnsubscribeRejected.into(),
        ])
    }
}

impl Sink for ApiEventPublisher {
    async fn consume(&mut self, input: Self::Input) -> ProcessorResult<()> {
        let receivers = self.publish(input);
        debug!(receivers, "registry api event publisher delivered event");
        Ok(())
    }
}

impl TryFrom<Event> for ApiEvent {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Api(event) => Ok(event),
            event => Err(ProcessorError::UnexpectedEvent {
                expected: "api event",
                actual: event.kind(),
            }),
        }
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
