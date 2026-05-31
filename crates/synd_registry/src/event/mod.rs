mod api_stream;
mod codec;
mod consumer;
mod domain;
mod journal;
mod runtime;
mod worker;

pub use api_stream::{ApiEventPublisher, ApiEventRecvError, ApiEventSubscriber};
pub use codec::{
    EncodedEvent, EventEncoding, EventEncodingError, EventEncodingResult, EventPayload,
};
pub use consumer::{
    Consumer, Processor, ProcessorError, ProcessorId, ProcessorInput, ProcessorResult,
    RecordedEvents, Sink,
};
pub use domain::{
    ApiEvent, ApiEventKind, ApiFeedSubscribeRejected, ApiFeedSubscribed,
    ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed, Event,
    EventInterests, EventKind, FeedSubscribed, FeedUnsubscribed, RequestEvent, RequestEventKind,
    RequestId, SubEvent, SubEventKind, SubscribeFeedRejected, SubscribeFeedRequested,
    SubscriptionChanged, SubscriptionLifecycle, UnsubscribeFeedRejected, UnsubscribeFeedRequested,
};
pub use journal::{
    EventCursor, EventCursorPos, EventJournal, EventJournalError, EventJournalResult,
    EventReadBatch, JournaledEvent,
};
pub use runtime::{EventSubmitter, EventSubmitterError, EventSubmitterResult};
pub use worker::{
    DrainOutcome, EventWakePublisher, EventWakeRecvError, EventWakeSubscriber, SinkWorker, Trigger,
    Worker, WorkerError, WorkerHandle, WorkerResult, WorkerSet,
};
