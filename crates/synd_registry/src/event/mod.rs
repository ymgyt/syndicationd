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
    ConsumerDispatch, ConsumerEventInput, ConsumerRegistry, EmptyConsumerRegistry, EventConsumer,
    EventConsumerError, EventConsumerId, EventConsumerResult, EventConsumerSession, RecordedEvents,
};
pub use domain::{
    ApiEvent, ApiEventKind, ApiFeedSubscribeRejected, ApiFeedSubscribed,
    ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed, CrawlEvent,
    CrawlEventKind, Event, EventKind, EventReadFilter, FeedSubscribed, FeedUnsubscribed,
    RequestEvent, RequestEventKind, RequestId, SubEvent, SubEventKind, SubscribeFeedRejected,
    SubscribeFeedRequested, SubscriptionChanged, SubscriptionLifecycle, UnsubscribeFeedRejected,
    UnsubscribeFeedRequested,
};
pub use journal::{
    EventCursor, EventCursorPos, EventJournal, EventJournalConsumer, EventJournalError,
    EventJournalExt, EventJournalResult, EventReadBatch, JournaledEvent,
};
pub use runtime::{
    EventRecorder, EventRuntime, EventRuntimeError, EventRuntimeOutput, EventRuntimeResult,
    EventSubmitter,
};
pub use worker::{
    DrainOutcome, EventWakePublisher, EventWakeRecvError, EventWakeSubmitter, EventWakeSubscriber,
    Trigger, Worker, WorkerError, WorkerHandle, WorkerResult, WorkerSet,
};
