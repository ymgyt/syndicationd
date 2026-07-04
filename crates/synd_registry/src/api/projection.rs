use chrono::{DateTime, Utc};

use crate::{
    api::{ApiEvent, ApiTimelineChanged},
    db::FeedRegistryDb,
    event::{
        Event, EventInput, EventType, Processor, ProcessorError, ProcessorId, ProcessorResult,
        Projector, RegistryEvent, TimelineChangedEvent,
    },
};

/// Event input used to project public API events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiEventProjectionInput {
    event: TimelineChangedEvent,
    occurred_at: DateTime<Utc>,
}

impl EventInput for ApiEventProjectionInput {
    const INTERESTS: &'static [EventType] = &[TimelineChangedEvent::TYPE];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::TimelineChanged(event) => Ok(Self { event, occurred_at }),
            event => Err(ProcessorError::unexpected_input(
                "api projection event",
                &event,
            )),
        }
    }
}

/// Projects timeline facts into public API events.
#[derive(Debug, Clone)]
pub struct ApiEventProj;

impl ApiEventProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiEventProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for ApiEventProj {
    type Input = ApiEventProjectionInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::ApiEventProjection
    }
}

impl<S> Projector<S> for ApiEventProj
where
    S: FeedRegistryDb,
{
    async fn project(
        &mut self,
        _tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        Ok(vec![
            ApiEvent::TimelineChanged(ApiTimelineChanged::new(
                input.event.timeline,
                input.occurred_at,
                input.event.affected_feeds,
            ))
            .into(),
        ])
    }
}
