mod codec;
mod domain;
mod journal;
mod processor;
mod reconcile;
mod recorder;
mod worker;

pub use codec::{EncodedEvent, EventEncoding, EventEncodingError, EventEncodingResult};
pub use domain::{
    CrawlJobFinishedEvent, CrawlRequestedEvent, CrawlTargetActivatedEvent,
    CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, EntryChangedEvent,
    EntryDiscoveredEvent, Event, EventInterests, EventType, FeedSubscribedEvent,
    FeedUnsubscribedEvent, RegistryEvent, SubEvent, SubscriptionChangedEvent, TimelineChangedEvent,
};
pub use journal::{
    EventCursor, EventCursorPos, EventJournal, EventJournalAppend, EventReadBatch, JournaledEvent,
};

pub use processor::{
    ClassifyError, EventInput, FailureClass, InputBatch, Processor, ProcessorError, ProcessorId,
    ProcessorResult, Projector, Reaction, RecordedEvents, Sink, WakeRequest,
};
pub(crate) use reconcile::{Reconciler, ReconcilerWorker};
pub use recorder::EventRecorder;
pub(crate) use worker::{EventLoop, EventWorker, JournalWorker, PostCommitWorker};
pub use worker::{
    EventWake, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, Trigger, WorkerError,
    WorkerHandle, WorkerId, WorkerResult, WorkerSet,
};
