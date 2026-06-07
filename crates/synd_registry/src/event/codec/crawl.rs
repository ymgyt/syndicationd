use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{
    CrawlEvent, CrawlEventKind, CrawlJobEnqueuedEvent, CrawlJobFinishedEvent, CrawlJobStartedEvent,
    CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, Event,
};

impl CrawlEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::TargetActivated(event) => encode_payload(event),
            Self::TargetPolicyChanged(event) => encode_payload(event),
            Self::TargetDeactivated(event) => encode_payload(event),
            Self::JobEnqueued(event) => encode_payload(event),
            Self::JobStarted(event) => encode_payload(event),
            Self::JobFinished(event) => encode_payload(event),
        }
    }
}

impl CrawlEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::TargetActivated => <CrawlTargetActivatedEvent as EventPayload>::EVENT_TYPE,
            Self::TargetPolicyChanged => {
                <CrawlTargetPolicyChangedEvent as EventPayload>::EVENT_TYPE
            }
            Self::TargetDeactivated => <CrawlTargetDeactivatedEvent as EventPayload>::EVENT_TYPE,
            Self::JobEnqueued => <CrawlJobEnqueuedEvent as EventPayload>::EVENT_TYPE,
            Self::JobStarted => <CrawlJobStartedEvent as EventPayload>::EVENT_TYPE,
            Self::JobFinished => <CrawlJobFinishedEvent as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for CrawlTargetActivatedEvent {
    const EVENT_TYPE: &'static str = event_type::CRAWL_TARGET_ACTIVATED;

    fn into_event(self) -> Event {
        Event::Crawl(CrawlEvent::TargetActivated(self))
    }
}

impl EventPayload for CrawlTargetPolicyChangedEvent {
    const EVENT_TYPE: &'static str = event_type::CRAWL_TARGET_POLICY_CHANGED;

    fn into_event(self) -> Event {
        Event::Crawl(CrawlEvent::TargetPolicyChanged(self))
    }
}

impl EventPayload for CrawlTargetDeactivatedEvent {
    const EVENT_TYPE: &'static str = event_type::CRAWL_TARGET_DEACTIVATED;

    fn into_event(self) -> Event {
        Event::Crawl(CrawlEvent::TargetDeactivated(self))
    }
}

impl EventPayload for CrawlJobEnqueuedEvent {
    const EVENT_TYPE: &'static str = event_type::CRAWL_JOB_ENQUEUED;

    fn into_event(self) -> Event {
        Event::Crawl(CrawlEvent::JobEnqueued(self))
    }
}

impl EventPayload for CrawlJobStartedEvent {
    const EVENT_TYPE: &'static str = event_type::CRAWL_JOB_STARTED;

    fn into_event(self) -> Event {
        Event::Crawl(CrawlEvent::JobStarted(self))
    }
}

impl EventPayload for CrawlJobFinishedEvent {
    const EVENT_TYPE: &'static str = event_type::CRAWL_JOB_FINISHED;

    fn into_event(self) -> Event {
        Event::Crawl(CrawlEvent::JobFinished(self))
    }
}
