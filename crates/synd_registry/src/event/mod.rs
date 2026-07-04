mod codec;
mod domain;
mod journal;
mod processor;
mod recorder;
mod worker;

pub use codec::{EncodedEvent, EventEncoding, EventEncodingError, EventEncodingResult};
pub use domain::{
    CrawlJobFinishedEvent, CrawlRequestedEvent, CrawlTargetActivatedEvent,
    CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, EntryChangedEvent,
    EntryDiscoveredEvent, Event, EventInterests, EventType, FeedChangedEvent, FeedDiscoveredEvent,
    FeedSubscribedEvent, FeedUnsubscribedEvent, RegistryEvent, SubEvent, SubscriptionChangedEvent,
    TimelineChangedEvent,
};
pub use journal::{
    EventCursor, EventCursorPos, EventJournal, EventJournalAppend, EventReadBatch, JournaledEvent,
};
pub(crate) use processor::skip_permanent_error;
pub use processor::{
    ClassifyError, EventInput, FailureClass, InputBatch, Processor, ProcessorError, ProcessorId,
    ProcessorResult, Projector, Reaction, Reconciler, RecordedEvents, Sink, WakeRequest,
};
pub(crate) use processor::{JournalHandler, ProjectorAdapter, ReconcilerAdapter};
pub use recorder::EventRecorder;
pub use worker::{
    EventWake, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, Trigger, WorkerError,
    WorkerHandle, WorkerId, WorkerResult, WorkerSet,
};
pub(crate) use worker::{EventWorker, JournalWorker, PostCommitWorker, spawn_event_loop};
