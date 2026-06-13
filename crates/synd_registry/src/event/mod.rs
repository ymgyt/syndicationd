mod codec;
mod domain;
mod journal;
mod processor;
mod recorder;
mod runtime;
mod worker;

pub use codec::{EncodedEvent, EventEncoding, EventEncodingError, EventEncodingResult};
pub use domain::{
    CrawlJobEnqueuedEvent, CrawlJobFinishedEvent, CrawlJobStartedEvent, CrawlTargetActivatedEvent,
    CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, EntryChangedEvent,
    EntryDiscoveredEvent, Event, EventInterests, EventPayloadError, EventType, FeedChangedEvent,
    FeedDiscoveredEvent, FeedSubscribedEvent, FeedUnsubscribedEvent, RegistryEvent, RequestId,
    SubscribeFeedRejected, SubscribeFeedRequested, SubscriptionChangedEvent, SubscriptionLifecycle,
    TimelineChangedEvent, UnsubscribeFeedRejected, UnsubscribeFeedRequested,
};
pub use journal::{
    EventCursor, EventCursorPos, EventReadBatch, JournalAppendTx, JournalTx, JournaledEvent,
};
pub(crate) use processor::skip_permanent_error;
pub use processor::{
    ClassifyError, ConsumeContext, Consumer, ConsumerInput, FailureClass, InputBatch, Processor,
    ProcessorError, ProcessorId, ProcessorResult, ReconcileContext, RecordedEvents,
    RegistryContext, Sink, SubscriberScope,
};
pub use recorder::{EventRecorder, JournalEventMeta};
pub use runtime::{EventSubmitter, EventSubmitterError, EventSubmitterResult};
pub(crate) use worker::{CursorAdapter, EventWorker, PostCommitAdapter, spawn_event_loop};
pub use worker::{
    EventWake, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, Trigger, WorkerError,
    WorkerHandle, WorkerId, WorkerResult, WorkerSet,
};
