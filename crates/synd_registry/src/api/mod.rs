//! API-facing event projection and stream delivery.

mod event;
mod projection;
mod publisher;

pub use event::{
    ApiCrawlJobEnqueued, ApiCrawlJobFinished, ApiCrawlJobStarted, ApiEntryChanged,
    ApiEntryDiscovered, ApiEvent, ApiFeedChanged, ApiFeedDiscovered, ApiFeedSubscribeRejected,
    ApiFeedSubscribed, ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed,
    ApiTimelineChanged,
};
pub use projection::{ApiEventProj, ApiEventProjectionInput};
pub use publisher::{ApiEventPublisher, ApiEventRecvError, ApiEventSubscriber};
