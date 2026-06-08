mod api_stream;
mod codec;
mod domain;
mod journal;
mod processor;
mod reconciler;
mod runtime;
mod worker;

pub use api_stream::{ApiEventPublisher, ApiEventRecvError, ApiEventSubscriber};
pub use codec::{
    EncodedEvent, EventEncoding, EventEncodingError, EventEncodingResult, EventPayload,
};
pub use domain::{
    ApiEvent, ApiEventKind, ApiFeedSubscribeRejected, ApiFeedSubscribed,
    ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed, CrawlEvent,
    CrawlEventKind, CrawlJobEnqueuedEvent, CrawlJobFinishedEvent, CrawlJobStartedEvent,
    CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, Event,
    EventInterests, EventKind, FeedChangedEvent, FeedDiscoveredEvent, FeedEvent, FeedEventKind,
    FeedSubscribedEvent, FeedUnsubscribedEvent, RequestEvent, RequestEventKind, RequestId,
    SubEvent, SubEventKind, SubscribeFeedRejected, SubscribeFeedRequested,
    SubscriptionChangedEvent, SubscriptionLifecycle, UnsubscribeFeedRejected,
    UnsubscribeFeedRequested,
};
pub use journal::{EventCursor, EventCursorPos, EventReadBatch, JournalTx, JournaledEvent};
pub use processor::{
    ConsumeContext, Consumer, PostCommit, Processor, ProcessorError, ProcessorId, ProcessorInput,
    ProcessorPhase, ProcessorResult, ReconcileContext, RecordedEvents, Sink, SubscriberScope,
    Transactional,
};
pub(crate) use reconciler::{Reconciler, spawn_reconciler_worker};
pub use runtime::{EventSubmitter, EventSubmitterError, EventSubmitterResult};
pub use worker::{
    EventWake, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, Trigger, WorkerError,
    WorkerHandle, WorkerId, WorkerResult, WorkerSet,
};
pub(crate) use worker::{WorkerPhase, spawn_worker};
