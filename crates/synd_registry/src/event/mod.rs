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
    ClassifyError, EventInput, EventReconciler, FailureClass, InputBatch, Processor,
    ProcessorError, ProcessorId, ProcessorResult, Projector, RecordedEvents, Sink,
};
pub(crate) use processor::{CursorProjector, CursorRole, EventReconcilerAdapter};
pub use recorder::EventRecorder;
pub(crate) use worker::{CursorAdapter, EventWorker, PostCommitAdapter, spawn_event_loop};
pub use worker::{
    EventWake, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, Trigger, WorkerError,
    WorkerHandle, WorkerId, WorkerResult, WorkerSet,
};
