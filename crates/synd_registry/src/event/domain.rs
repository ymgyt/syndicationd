use std::fmt;

use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use synd_feed::types::{Category, EntryId, FeedUrl, Requirement};
use thiserror::Error;

use crate::{
    api::{
        ApiCrawlJobEnqueued, ApiCrawlJobFinished, ApiCrawlJobStarted, ApiEntryChanged,
        ApiEntryDiscovered, ApiEvent, ApiFeedChanged, ApiFeedDiscovered, ApiFeedSubscribeRejected,
        ApiFeedSubscribed, ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected,
        ApiFeedUnsubscribed, ApiTimelineChanged,
    },
    crawl::{
        job::{CrawlJob, CrawlJobId, CrawlJobQueueLane, CrawlJobTrigger},
        policy::CrawlPolicy,
    },
    subscription::SubscriptionKey,
    timeline::TimelineKey,
};

/// Stable identifier stored in the event journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    SubscribeFeedRequested,
    SubscribeFeedRejected,
    UnsubscribeFeedRequested,
    UnsubscribeFeedRejected,
    FeedSubscribed,
    SubscriptionChanged,
    FeedUnsubscribed,
    CrawlTargetActivated,
    CrawlTargetPolicyChanged,
    CrawlTargetDeactivated,
    CrawlJobEnqueued,
    CrawlJobStarted,
    CrawlJobFinished,
    FeedDiscovered,
    FeedChanged,
    EntryDiscovered,
    EntryChanged,
    TimelineChanged,
    ApiFeedSubscribed,
    ApiFeedSubscribeRejected,
    ApiFeedSubscriptionChanged,
    ApiFeedUnsubscribed,
    ApiFeedUnsubscribeRejected,
    ApiCrawlJobEnqueued,
    ApiCrawlJobStarted,
    ApiCrawlJobFinished,
    ApiFeedDiscovered,
    ApiFeedChanged,
    ApiEntryDiscovered,
    ApiEntryChanged,
    ApiTimelineChanged,
}

impl EventType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SubscribeFeedRequested => "request.subscribe_feed.requested",
            Self::SubscribeFeedRejected => "request.subscribe_feed.rejected",
            Self::UnsubscribeFeedRequested => "request.unsubscribe_feed.requested",
            Self::UnsubscribeFeedRejected => "request.unsubscribe_feed.rejected",
            Self::FeedSubscribed => "sub.feed.subscribed",
            Self::SubscriptionChanged => "sub.subscription.changed",
            Self::FeedUnsubscribed => "sub.feed.unsubscribed",
            Self::CrawlTargetActivated => "crawl.target.activated",
            Self::CrawlTargetPolicyChanged => "crawl.target.policy_changed",
            Self::CrawlTargetDeactivated => "crawl.target.deactivated",
            Self::CrawlJobEnqueued => "crawl.job.enqueued",
            Self::CrawlJobStarted => "crawl.job.started",
            Self::CrawlJobFinished => "crawl.job.finished",
            Self::FeedDiscovered => "feed.discovered",
            Self::FeedChanged => "feed.changed",
            Self::EntryDiscovered => "entry.discovered",
            Self::EntryChanged => "entry.changed",
            Self::TimelineChanged => "timeline.changed",
            Self::ApiFeedSubscribed => "api.feed.subscribed",
            Self::ApiFeedSubscribeRejected => "api.feed.subscribe_rejected",
            Self::ApiFeedSubscriptionChanged => "api.feed.subscription.changed",
            Self::ApiFeedUnsubscribed => "api.feed.unsubscribed",
            Self::ApiFeedUnsubscribeRejected => "api.feed.unsubscribe_rejected",
            Self::ApiCrawlJobEnqueued => "api.crawl.job.enqueued",
            Self::ApiCrawlJobStarted => "api.crawl.job.started",
            Self::ApiCrawlJobFinished => "api.crawl.job.finished",
            Self::ApiFeedDiscovered => "api.feed.discovered",
            Self::ApiFeedChanged => "api.feed.changed",
            Self::ApiEntryDiscovered => "api.entry.discovered",
            Self::ApiEntryChanged => "api.entry.changed",
            Self::ApiTimelineChanged => "api.timeline.changed",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "request.subscribe_feed.requested" => Some(Self::SubscribeFeedRequested),
            "request.subscribe_feed.rejected" => Some(Self::SubscribeFeedRejected),
            "request.unsubscribe_feed.requested" => Some(Self::UnsubscribeFeedRequested),
            "request.unsubscribe_feed.rejected" => Some(Self::UnsubscribeFeedRejected),
            "sub.feed.subscribed" => Some(Self::FeedSubscribed),
            "sub.subscription.changed" => Some(Self::SubscriptionChanged),
            "sub.feed.unsubscribed" => Some(Self::FeedUnsubscribed),
            "crawl.target.activated" => Some(Self::CrawlTargetActivated),
            "crawl.target.policy_changed" => Some(Self::CrawlTargetPolicyChanged),
            "crawl.target.deactivated" => Some(Self::CrawlTargetDeactivated),
            "crawl.job.enqueued" => Some(Self::CrawlJobEnqueued),
            "crawl.job.started" => Some(Self::CrawlJobStarted),
            "crawl.job.finished" => Some(Self::CrawlJobFinished),
            "feed.discovered" => Some(Self::FeedDiscovered),
            "feed.changed" => Some(Self::FeedChanged),
            "entry.discovered" => Some(Self::EntryDiscovered),
            "entry.changed" => Some(Self::EntryChanged),
            "timeline.changed" => Some(Self::TimelineChanged),
            "api.feed.subscribed" => Some(Self::ApiFeedSubscribed),
            "api.feed.subscribe_rejected" => Some(Self::ApiFeedSubscribeRejected),
            "api.feed.subscription.changed" => Some(Self::ApiFeedSubscriptionChanged),
            "api.feed.unsubscribed" => Some(Self::ApiFeedUnsubscribed),
            "api.feed.unsubscribe_rejected" => Some(Self::ApiFeedUnsubscribeRejected),
            "api.crawl.job.enqueued" => Some(Self::ApiCrawlJobEnqueued),
            "api.crawl.job.started" => Some(Self::ApiCrawlJobStarted),
            "api.crawl.job.finished" => Some(Self::ApiCrawlJobFinished),
            "api.feed.discovered" => Some(Self::ApiFeedDiscovered),
            "api.feed.changed" => Some(Self::ApiFeedChanged),
            "api.entry.discovered" => Some(Self::ApiEntryDiscovered),
            "api.entry.changed" => Some(Self::ApiEntryChanged),
            "api.timeline.changed" => Some(Self::ApiTimelineChanged),
            _ => None,
        }
    }
}

/// Event payload persisted in the registry journal.
pub trait RegistryEvent: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    const TYPE: EventType;
}

/// A typed fact recorded in the registry event journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    SubscribeFeedRequested(SubscribeFeedRequested),
    SubscribeFeedRejected(SubscribeFeedRejected),
    UnsubscribeFeedRequested(UnsubscribeFeedRequested),
    UnsubscribeFeedRejected(UnsubscribeFeedRejected),
    FeedSubscribed(FeedSubscribedEvent),
    SubscriptionChanged(SubscriptionChangedEvent),
    FeedUnsubscribed(FeedUnsubscribedEvent),
    CrawlTargetActivated(CrawlTargetActivatedEvent),
    CrawlTargetPolicyChanged(CrawlTargetPolicyChangedEvent),
    CrawlTargetDeactivated(CrawlTargetDeactivatedEvent),
    CrawlJobEnqueued(CrawlJobEnqueuedEvent),
    CrawlJobStarted(CrawlJobStartedEvent),
    CrawlJobFinished(CrawlJobFinishedEvent),
    FeedDiscovered(FeedDiscoveredEvent),
    FeedChanged(FeedChangedEvent),
    EntryDiscovered(EntryDiscoveredEvent),
    EntryChanged(EntryChangedEvent),
    TimelineChanged(TimelineChangedEvent),
    ApiFeedSubscribed(ApiFeedSubscribed),
    ApiFeedSubscribeRejected(ApiFeedSubscribeRejected),
    ApiFeedSubscriptionChanged(ApiFeedSubscriptionChanged),
    ApiFeedUnsubscribed(ApiFeedUnsubscribed),
    ApiFeedUnsubscribeRejected(ApiFeedUnsubscribeRejected),
    ApiCrawlJobEnqueued(ApiCrawlJobEnqueued),
    ApiCrawlJobStarted(ApiCrawlJobStarted),
    ApiCrawlJobFinished(ApiCrawlJobFinished),
    ApiFeedDiscovered(ApiFeedDiscovered),
    ApiFeedChanged(ApiFeedChanged),
    ApiEntryDiscovered(ApiEntryDiscovered),
    ApiEntryChanged(ApiEntryChanged),
    ApiTimelineChanged(ApiTimelineChanged),
}

impl Event {
    pub fn event_type(&self) -> EventType {
        match self {
            Self::SubscribeFeedRequested(_) => EventType::SubscribeFeedRequested,
            Self::SubscribeFeedRejected(_) => EventType::SubscribeFeedRejected,
            Self::UnsubscribeFeedRequested(_) => EventType::UnsubscribeFeedRequested,
            Self::UnsubscribeFeedRejected(_) => EventType::UnsubscribeFeedRejected,
            Self::FeedSubscribed(_) => EventType::FeedSubscribed,
            Self::SubscriptionChanged(_) => EventType::SubscriptionChanged,
            Self::FeedUnsubscribed(_) => EventType::FeedUnsubscribed,
            Self::CrawlTargetActivated(_) => EventType::CrawlTargetActivated,
            Self::CrawlTargetPolicyChanged(_) => EventType::CrawlTargetPolicyChanged,
            Self::CrawlTargetDeactivated(_) => EventType::CrawlTargetDeactivated,
            Self::CrawlJobEnqueued(_) => EventType::CrawlJobEnqueued,
            Self::CrawlJobStarted(_) => EventType::CrawlJobStarted,
            Self::CrawlJobFinished(_) => EventType::CrawlJobFinished,
            Self::FeedDiscovered(_) => EventType::FeedDiscovered,
            Self::FeedChanged(_) => EventType::FeedChanged,
            Self::EntryDiscovered(_) => EventType::EntryDiscovered,
            Self::EntryChanged(_) => EventType::EntryChanged,
            Self::TimelineChanged(_) => EventType::TimelineChanged,
            Self::ApiFeedSubscribed(_) => EventType::ApiFeedSubscribed,
            Self::ApiFeedSubscribeRejected(_) => EventType::ApiFeedSubscribeRejected,
            Self::ApiFeedSubscriptionChanged(_) => EventType::ApiFeedSubscriptionChanged,
            Self::ApiFeedUnsubscribed(_) => EventType::ApiFeedUnsubscribed,
            Self::ApiFeedUnsubscribeRejected(_) => EventType::ApiFeedUnsubscribeRejected,
            Self::ApiCrawlJobEnqueued(_) => EventType::ApiCrawlJobEnqueued,
            Self::ApiCrawlJobStarted(_) => EventType::ApiCrawlJobStarted,
            Self::ApiCrawlJobFinished(_) => EventType::ApiCrawlJobFinished,
            Self::ApiFeedDiscovered(_) => EventType::ApiFeedDiscovered,
            Self::ApiFeedChanged(_) => EventType::ApiFeedChanged,
            Self::ApiEntryDiscovered(_) => EventType::ApiEntryDiscovered,
            Self::ApiEntryChanged(_) => EventType::ApiEntryChanged,
            Self::ApiTimelineChanged(_) => EventType::ApiTimelineChanged,
        }
    }

    fn payload_error<T>(&self) -> EventPayloadError
    where
        T: RegistryEvent,
    {
        EventPayloadError::new(T::TYPE, self.event_type())
    }
}

impl From<ApiEvent> for Event {
    fn from(event: ApiEvent) -> Self {
        match event {
            ApiEvent::FeedSubscribed(event) => Self::ApiFeedSubscribed(event),
            ApiEvent::FeedSubscribeRejected(event) => Self::ApiFeedSubscribeRejected(event),
            ApiEvent::FeedSubscriptionChanged(event) => Self::ApiFeedSubscriptionChanged(event),
            ApiEvent::FeedUnsubscribed(event) => Self::ApiFeedUnsubscribed(event),
            ApiEvent::FeedUnsubscribeRejected(event) => Self::ApiFeedUnsubscribeRejected(event),
            ApiEvent::CrawlJobEnqueued(event) => Self::ApiCrawlJobEnqueued(event),
            ApiEvent::CrawlJobStarted(event) => Self::ApiCrawlJobStarted(event),
            ApiEvent::CrawlJobFinished(event) => Self::ApiCrawlJobFinished(event),
            ApiEvent::FeedDiscovered(event) => Self::ApiFeedDiscovered(event),
            ApiEvent::FeedChanged(event) => Self::ApiFeedChanged(event),
            ApiEvent::EntryDiscovered(event) => Self::ApiEntryDiscovered(event),
            ApiEvent::EntryChanged(event) => Self::ApiEntryChanged(event),
            ApiEvent::TimelineChanged(event) => Self::ApiTimelineChanged(event),
        }
    }
}

impl From<ApiFeedSubscribed> for Event {
    fn from(event: ApiFeedSubscribed) -> Self {
        Self::ApiFeedSubscribed(event)
    }
}

impl From<ApiFeedSubscribeRejected> for Event {
    fn from(event: ApiFeedSubscribeRejected) -> Self {
        Self::ApiFeedSubscribeRejected(event)
    }
}

impl From<ApiFeedSubscriptionChanged> for Event {
    fn from(event: ApiFeedSubscriptionChanged) -> Self {
        Self::ApiFeedSubscriptionChanged(event)
    }
}

impl From<ApiFeedUnsubscribed> for Event {
    fn from(event: ApiFeedUnsubscribed) -> Self {
        Self::ApiFeedUnsubscribed(event)
    }
}

impl From<ApiFeedUnsubscribeRejected> for Event {
    fn from(event: ApiFeedUnsubscribeRejected) -> Self {
        Self::ApiFeedUnsubscribeRejected(event)
    }
}

impl From<ApiCrawlJobEnqueued> for Event {
    fn from(event: ApiCrawlJobEnqueued) -> Self {
        Self::ApiCrawlJobEnqueued(event)
    }
}

impl From<ApiCrawlJobStarted> for Event {
    fn from(event: ApiCrawlJobStarted) -> Self {
        Self::ApiCrawlJobStarted(event)
    }
}

impl From<ApiCrawlJobFinished> for Event {
    fn from(event: ApiCrawlJobFinished) -> Self {
        Self::ApiCrawlJobFinished(event)
    }
}

impl From<ApiFeedDiscovered> for Event {
    fn from(event: ApiFeedDiscovered) -> Self {
        Self::ApiFeedDiscovered(event)
    }
}

impl From<ApiFeedChanged> for Event {
    fn from(event: ApiFeedChanged) -> Self {
        Self::ApiFeedChanged(event)
    }
}

impl From<ApiEntryDiscovered> for Event {
    fn from(event: ApiEntryDiscovered) -> Self {
        Self::ApiEntryDiscovered(event)
    }
}

impl From<ApiEntryChanged> for Event {
    fn from(event: ApiEntryChanged) -> Self {
        Self::ApiEntryChanged(event)
    }
}

impl From<ApiTimelineChanged> for Event {
    fn from(event: ApiTimelineChanged) -> Self {
        Self::ApiTimelineChanged(event)
    }
}

/// Error returned when a flat journal event is converted to the wrong payload type.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unexpected event payload: expected {expected:?}, actual {actual:?}")]
pub struct EventPayloadError {
    pub expected: EventType,
    pub actual: EventType,
}

impl EventPayloadError {
    pub fn new(expected: EventType, actual: EventType) -> Self {
        Self { expected, actual }
    }
}

/// Event categories a worker is interested in consuming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInterests {
    types: Vec<EventType>,
}

impl EventInterests {
    pub fn new(types: impl Into<Vec<EventType>>) -> Self {
        Self {
            types: types.into(),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn types(&self) -> &[EventType] {
        &self.types
    }

    pub fn contains(&self, event_type: EventType) -> bool {
        self.types.contains(&event_type)
    }

    pub fn matches_any(&self, event_types: &[EventType]) -> bool {
        event_types
            .iter()
            .any(|event_type| self.contains(*event_type))
    }
}

/// Stable identity for correlating accepted registry requests with async outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(String);

impl RequestId {
    pub fn generate() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::rng(), 24))
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A request to start a subscription relation was accepted for async processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeFeedRequested {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
}

impl SubscribeFeedRequested {
    pub fn new(
        request_id: RequestId,
        subscription: SubscriptionKey,
        requirement: Option<Requirement>,
        category: Option<Category<'static>>,
        crawl_policy: CrawlPolicy,
    ) -> Self {
        Self {
            request_id,
            subscription,
            requirement,
            category,
            crawl_policy,
        }
    }
}

impl From<SubscribeFeedRequested> for Event {
    fn from(event: SubscribeFeedRequested) -> Self {
        Self::SubscribeFeedRequested(event)
    }
}

/// A request to start a subscription relation was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeFeedRejected {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub reason: String,
}

impl SubscribeFeedRejected {
    pub fn new(
        request_id: RequestId,
        subscription: SubscriptionKey,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            subscription,
            reason: reason.into(),
        }
    }
}

impl From<SubscribeFeedRejected> for Event {
    fn from(event: SubscribeFeedRejected) -> Self {
        Self::SubscribeFeedRejected(event)
    }
}

/// A request to end a subscription relation was accepted for async processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeFeedRequested {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl UnsubscribeFeedRequested {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

impl From<UnsubscribeFeedRequested> for Event {
    fn from(event: UnsubscribeFeedRequested) -> Self {
        Self::UnsubscribeFeedRequested(event)
    }
}

/// A request to end a subscription relation was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeFeedRejected {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub reason: String,
}

impl UnsubscribeFeedRejected {
    pub fn new(
        request_id: RequestId,
        subscription: SubscriptionKey,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            subscription,
            reason: reason.into(),
        }
    }
}

impl From<UnsubscribeFeedRejected> for Event {
    fn from(event: UnsubscribeFeedRejected) -> Self {
        Self::UnsubscribeFeedRejected(event)
    }
}

/// A domain fact about the lifecycle of one feed subscription relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionLifecycle {
    /// The subscriber started subscribing to the feed.
    Subscribed(FeedSubscribedEvent),
    /// An active subscription changed its registry-owned attributes.
    Changed(SubscriptionChangedEvent),
    /// The subscriber stopped subscribing to the feed.
    Unsubscribed(FeedUnsubscribedEvent),
}

impl SubscriptionLifecycle {
    pub fn affected_feed_url(&self) -> &FeedUrl {
        match self {
            Self::Subscribed(event) => &event.subscription.feed_url,
            Self::Changed(event) => &event.subscription.feed_url,
            Self::Unsubscribed(event) => &event.subscription.feed_url,
        }
    }
}

/// A subscription relation was created and became active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedSubscribedEvent {
    /// The subscription relation that was created.
    pub subscription: SubscriptionKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

impl FeedSubscribedEvent {
    pub fn new(subscription: SubscriptionKey) -> Self {
        Self {
            subscription,
            request_id: None,
        }
    }

    #[must_use]
    pub fn with_request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl From<FeedSubscribedEvent> for Event {
    fn from(event: FeedSubscribedEvent) -> Self {
        Self::FeedSubscribed(event)
    }
}

/// An active subscription was updated without ending the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionChangedEvent {
    /// The subscription relation whose registry-owned attributes changed.
    pub subscription: SubscriptionKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

impl SubscriptionChangedEvent {
    pub fn new(subscription: SubscriptionKey) -> Self {
        Self {
            subscription,
            request_id: None,
        }
    }

    #[must_use]
    pub fn with_request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl From<SubscriptionChangedEvent> for Event {
    fn from(event: SubscriptionChangedEvent) -> Self {
        Self::SubscriptionChanged(event)
    }
}

/// A subscription relation was ended and is no longer active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedUnsubscribedEvent {
    /// The subscription relation that ended.
    pub subscription: SubscriptionKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

impl FeedUnsubscribedEvent {
    pub fn new(subscription: SubscriptionKey) -> Self {
        Self {
            subscription,
            request_id: None,
        }
    }

    #[must_use]
    pub fn with_request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }
}

impl From<FeedUnsubscribedEvent> for Event {
    fn from(event: FeedUnsubscribedEvent) -> Self {
        Self::FeedUnsubscribed(event)
    }
}

/// A crawl target became active and should be considered by the scheduler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlTargetActivatedEvent {
    pub feed_url: FeedUrl,
    pub policy: CrawlPolicy,
}

impl CrawlTargetActivatedEvent {
    pub fn new(feed_url: FeedUrl, policy: CrawlPolicy) -> Self {
        Self { feed_url, policy }
    }
}

impl From<CrawlTargetActivatedEvent> for Event {
    fn from(event: CrawlTargetActivatedEvent) -> Self {
        Self::CrawlTargetActivated(event)
    }
}

/// An active crawl target's effective policy changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlTargetPolicyChangedEvent {
    pub feed_url: FeedUrl,
    pub policy: CrawlPolicy,
}

impl CrawlTargetPolicyChangedEvent {
    pub fn new(feed_url: FeedUrl, policy: CrawlPolicy) -> Self {
        Self { feed_url, policy }
    }
}

impl From<CrawlTargetPolicyChangedEvent> for Event {
    fn from(event: CrawlTargetPolicyChangedEvent) -> Self {
        Self::CrawlTargetPolicyChanged(event)
    }
}

/// A crawl target became inactive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlTargetDeactivatedEvent {
    pub feed_url: FeedUrl,
}

impl CrawlTargetDeactivatedEvent {
    pub fn new(feed_url: FeedUrl) -> Self {
        Self { feed_url }
    }
}

impl From<CrawlTargetDeactivatedEvent> for Event {
    fn from(event: CrawlTargetDeactivatedEvent) -> Self {
        Self::CrawlTargetDeactivated(event)
    }
}

/// A durable crawl job was created and can wake crawl workers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlJobEnqueuedEvent {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
    pub trigger: CrawlJobTrigger,
    pub queue: CrawlJobQueueLane,
    pub priority: i64,
    pub run_after: DateTime<Utc>,
}

impl CrawlJobEnqueuedEvent {
    pub fn new(
        job_id: CrawlJobId,
        feed_url: FeedUrl,
        trigger: CrawlJobTrigger,
        queue: CrawlJobQueueLane,
        priority: i64,
        run_after: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id,
            feed_url,
            trigger,
            queue,
            priority,
            run_after,
        }
    }
}

impl From<CrawlJob> for CrawlJobEnqueuedEvent {
    fn from(job: CrawlJob) -> Self {
        Self::new(
            job.job_id,
            job.feed_url,
            job.trigger,
            job.queue,
            job.priority,
            job.run_after,
        )
    }
}

impl From<CrawlJobEnqueuedEvent> for Event {
    fn from(event: CrawlJobEnqueuedEvent) -> Self {
        Self::CrawlJobEnqueued(event)
    }
}

/// A crawl job moved from pending to running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlJobStartedEvent {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
}

impl CrawlJobStartedEvent {
    pub fn new(job_id: CrawlJobId, feed_url: FeedUrl) -> Self {
        Self { job_id, feed_url }
    }
}

impl From<CrawlJob> for CrawlJobStartedEvent {
    fn from(job: CrawlJob) -> Self {
        Self::new(job.job_id, job.feed_url)
    }
}

impl From<CrawlJobStartedEvent> for Event {
    fn from(event: CrawlJobStartedEvent) -> Self {
        Self::CrawlJobStarted(event)
    }
}

/// A crawl job completed and moved out of the running set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlJobFinishedEvent {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
}

impl CrawlJobFinishedEvent {
    pub fn new(job_id: CrawlJobId, feed_url: FeedUrl) -> Self {
        Self { job_id, feed_url }
    }
}

impl From<CrawlJob> for CrawlJobFinishedEvent {
    fn from(job: CrawlJob) -> Self {
        Self::new(job.job_id, job.feed_url)
    }
}

impl From<CrawlJobFinishedEvent> for Event {
    fn from(event: CrawlJobFinishedEvent) -> Self {
        Self::CrawlJobFinished(event)
    }
}

/// A feed became known to the registry for the first time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedDiscoveredEvent {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
}

impl FeedDiscoveredEvent {
    /// Creates an event for a feed first observed by the registry.
    pub fn new(feed_url: FeedUrl, crawl_job_id: CrawlJobId) -> Self {
        Self {
            feed_url,
            crawl_job_id,
        }
    }
}

impl From<FeedDiscoveredEvent> for Event {
    fn from(event: FeedDiscoveredEvent) -> Self {
        Self::FeedDiscovered(event)
    }
}

/// The current state of a known feed changed after a crawl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedChangedEvent {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
}

impl FeedChangedEvent {
    /// Creates an event for a changed feed observed by a crawl job.
    pub fn new(feed_url: FeedUrl, crawl_job_id: CrawlJobId) -> Self {
        Self {
            feed_url,
            crawl_job_id,
        }
    }
}

impl From<FeedChangedEvent> for Event {
    fn from(event: FeedChangedEvent) -> Self {
        Self::FeedChanged(event)
    }
}

/// An entry became known to the registry for the first time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryDiscoveredEvent {
    pub feed_url: FeedUrl,
    pub entry_id: EntryId,
    pub crawl_job_id: CrawlJobId,
}

impl EntryDiscoveredEvent {
    /// Creates an event for an entry first observed by the registry.
    pub fn new(feed_url: FeedUrl, entry_id: EntryId, crawl_job_id: CrawlJobId) -> Self {
        Self {
            feed_url,
            entry_id,
            crawl_job_id,
        }
    }
}

impl From<EntryDiscoveredEvent> for Event {
    fn from(event: EntryDiscoveredEvent) -> Self {
        Self::EntryDiscovered(event)
    }
}

/// The current state of a known entry changed after a feed source was applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryChangedEvent {
    pub feed_url: FeedUrl,
    pub entry_id: EntryId,
    pub crawl_job_id: CrawlJobId,
}

impl EntryChangedEvent {
    /// Creates an event for a changed entry observed by a feed source.
    pub fn new(feed_url: FeedUrl, entry_id: EntryId, crawl_job_id: CrawlJobId) -> Self {
        Self {
            feed_url,
            entry_id,
            crawl_job_id,
        }
    }
}

impl From<EntryChangedEvent> for Event {
    fn from(event: EntryChangedEvent) -> Self {
        Self::EntryChanged(event)
    }
}

/// Timeline membership changed for a subscriber-visible timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineChangedEvent {
    pub timeline: TimelineKey,
    pub changed_at: DateTime<Utc>,
    pub affected_feeds: Vec<FeedUrl>,
}

impl TimelineChangedEvent {
    pub fn new(
        timeline: TimelineKey,
        changed_at: DateTime<Utc>,
        affected_feeds: Vec<FeedUrl>,
    ) -> Self {
        Self {
            timeline,
            changed_at,
            affected_feeds,
        }
    }
}

impl From<TimelineChangedEvent> for Event {
    fn from(event: TimelineChangedEvent) -> Self {
        Self::TimelineChanged(event)
    }
}

impl RegistryEvent for SubscribeFeedRequested {
    const TYPE: EventType = EventType::SubscribeFeedRequested;
}

impl RegistryEvent for SubscribeFeedRejected {
    const TYPE: EventType = EventType::SubscribeFeedRejected;
}

impl RegistryEvent for UnsubscribeFeedRequested {
    const TYPE: EventType = EventType::UnsubscribeFeedRequested;
}

impl RegistryEvent for UnsubscribeFeedRejected {
    const TYPE: EventType = EventType::UnsubscribeFeedRejected;
}

impl RegistryEvent for FeedSubscribedEvent {
    const TYPE: EventType = EventType::FeedSubscribed;
}

impl RegistryEvent for SubscriptionChangedEvent {
    const TYPE: EventType = EventType::SubscriptionChanged;
}

impl RegistryEvent for FeedUnsubscribedEvent {
    const TYPE: EventType = EventType::FeedUnsubscribed;
}

impl RegistryEvent for CrawlTargetActivatedEvent {
    const TYPE: EventType = EventType::CrawlTargetActivated;
}

impl RegistryEvent for CrawlTargetPolicyChangedEvent {
    const TYPE: EventType = EventType::CrawlTargetPolicyChanged;
}

impl RegistryEvent for CrawlTargetDeactivatedEvent {
    const TYPE: EventType = EventType::CrawlTargetDeactivated;
}

impl RegistryEvent for CrawlJobEnqueuedEvent {
    const TYPE: EventType = EventType::CrawlJobEnqueued;
}

impl RegistryEvent for CrawlJobStartedEvent {
    const TYPE: EventType = EventType::CrawlJobStarted;
}

impl RegistryEvent for CrawlJobFinishedEvent {
    const TYPE: EventType = EventType::CrawlJobFinished;
}

impl RegistryEvent for FeedDiscoveredEvent {
    const TYPE: EventType = EventType::FeedDiscovered;
}

impl RegistryEvent for FeedChangedEvent {
    const TYPE: EventType = EventType::FeedChanged;
}

impl RegistryEvent for EntryDiscoveredEvent {
    const TYPE: EventType = EventType::EntryDiscovered;
}

impl RegistryEvent for EntryChangedEvent {
    const TYPE: EventType = EventType::EntryChanged;
}

impl RegistryEvent for TimelineChangedEvent {
    const TYPE: EventType = EventType::TimelineChanged;
}

impl TryFrom<Event> for SubscribeFeedRequested {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::SubscribeFeedRequested(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for SubscribeFeedRejected {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::SubscribeFeedRejected(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for UnsubscribeFeedRequested {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::UnsubscribeFeedRequested(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for UnsubscribeFeedRejected {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::UnsubscribeFeedRejected(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for FeedSubscribedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::FeedSubscribed(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for SubscriptionChangedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::SubscriptionChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for FeedUnsubscribedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::FeedUnsubscribed(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for CrawlTargetActivatedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::CrawlTargetActivated(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for CrawlTargetPolicyChangedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::CrawlTargetPolicyChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for CrawlTargetDeactivatedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::CrawlTargetDeactivated(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for CrawlJobEnqueuedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::CrawlJobEnqueued(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for CrawlJobStartedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::CrawlJobStarted(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for CrawlJobFinishedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::CrawlJobFinished(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for FeedDiscoveredEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::FeedDiscovered(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for FeedChangedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::FeedChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for EntryDiscoveredEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::EntryDiscovered(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for EntryChangedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::EntryChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for TimelineChangedEvent {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::TimelineChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for ApiFeedSubscribed {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ApiFeedSubscribed(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for ApiFeedSubscribeRejected {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ApiFeedSubscribeRejected(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for ApiFeedSubscriptionChanged {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ApiFeedSubscriptionChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for ApiFeedUnsubscribed {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ApiFeedUnsubscribed(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for ApiFeedUnsubscribeRejected {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ApiFeedUnsubscribeRejected(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}

impl TryFrom<Event> for ApiTimelineChanged {
    type Error = EventPayloadError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::ApiTimelineChanged(payload) => Ok(payload),
            event => Err(event.payload_error::<Self>()),
        }
    }
}
