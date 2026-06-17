use std::fmt;

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;
use tracing::debug;

use crate::{
    SubscriberId,
    api::{ApiEvent, ApiTimelineChanged},
    event::{
        Event, EventInput, EventType, Processor, ProcessorError, ProcessorId, ProcessorResult,
        RegistryEvent, Sink,
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
}

impl Sink for ApiEventPublisher {
    async fn deliver(&mut self, input: Self::Input) {
        let receivers = self.publish(input);
        debug!(receivers, "registry api event publisher delivered event");
    }
}

impl EventInput for ApiEvent {
    const INTERESTS: &'static [EventType] = &[ApiTimelineChanged::TYPE];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::ApiTimelineChanged(event) => Ok(Self::TimelineChanged(event)),
            event => Err(ProcessorError::unexpected_input("api event", &event)),
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
            if event.subscriber_id() == &self.subscriber_id {
                return Ok(event);
            }
        }
    }
}
