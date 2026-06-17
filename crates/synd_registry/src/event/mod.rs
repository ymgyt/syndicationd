mod codec;
mod domain;
mod journal;
mod processor;
mod recorder;
mod worker;

pub use codec::{EncodedEvent, EventEncoding, EventEncodingError, EventEncodingResult};
pub use domain::{
    CrawlJobEnqueuedEvent, CrawlJobFinishedEvent, CrawlJobStartedEvent, CrawlTargetActivatedEvent,
    CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, EntryChangedEvent,
    EntryDiscoveredEvent, Event, EventInterests, EventType, FeedChangedEvent, FeedDiscoveredEvent,
    FeedSubscribedEvent, FeedUnsubscribedEvent, RegistryEvent, SubscriptionChangedEvent,
    SubscriptionLifecycle, TimelineChangedEvent,
};
pub use journal::{
    EventCursor, EventCursorPos, EventJournal, EventJournalAppend, EventReadBatch, JournaledEvent,
};
pub(crate) use processor::skip_permanent_error;
pub use processor::{
    ClassifyError, EventInput, FailureClass, InputBatch, Processor, ProcessorError, ProcessorId,
    ProcessorResult, Projector, Reconciler, RecordedEvents, Sink,
};
pub(crate) use processor::{CursorProjector, CursorReconciler, CursorRole};
pub use recorder::EventRecorder;
pub(crate) use worker::{
    CursorAdapter, EventWorker, PostCommitAdapter, ScanAdapter, spawn_event_loop,
};
pub use worker::{
    EventWake, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, Trigger, WorkerError,
    WorkerHandle, WorkerId, WorkerResult, WorkerSet,
};
