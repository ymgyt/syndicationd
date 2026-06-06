use std::fmt;

use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{
    crawl::{
        job::{CrawlJob, CrawlJobId, CrawlJobQueueLane, CrawlJobTrigger},
        policy::CrawlPolicy,
    },
    subscription::SubscriptionKey,
};

/// A typed fact recorded in the registry event journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Request(RequestEvent),
    Sub(SubEvent),
    Crawl(CrawlEvent),
    Api(ApiEvent),
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::Request(event) => event.kind().into(),
            Self::Sub(event) => event.kind().into(),
            Self::Crawl(event) => event.kind().into(),
            Self::Api(event) => event.kind().into(),
        }
    }
}

impl From<RequestEvent> for Event {
    fn from(event: RequestEvent) -> Self {
        Self::Request(event)
    }
}

impl From<SubEvent> for Event {
    fn from(event: SubEvent) -> Self {
        Self::Sub(event)
    }
}

impl From<CrawlEvent> for Event {
    fn from(event: CrawlEvent) -> Self {
        Self::Crawl(event)
    }
}

impl From<ApiEvent> for Event {
    fn from(event: ApiEvent) -> Self {
        Self::Api(event)
    }
}

/// A stable event category used to route committed facts to interested workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    Request(RequestEventKind),
    Sub(SubEventKind),
    Crawl(CrawlEventKind),
    Api(ApiEventKind),
}

impl From<RequestEventKind> for EventKind {
    fn from(kind: RequestEventKind) -> Self {
        Self::Request(kind)
    }
}

impl From<SubEventKind> for EventKind {
    fn from(kind: SubEventKind) -> Self {
        Self::Sub(kind)
    }
}

impl From<CrawlEventKind> for EventKind {
    fn from(kind: CrawlEventKind) -> Self {
        Self::Crawl(kind)
    }
}

impl From<ApiEventKind> for EventKind {
    fn from(kind: ApiEventKind) -> Self {
        Self::Api(kind)
    }
}

/// A stable request event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestEventKind {
    SubscribeFeedRequested,
    SubscribeFeedRejected,
    UnsubscribeFeedRequested,
    UnsubscribeFeedRejected,
}

/// A stable subscription event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubEventKind {
    FeedSubscribed,
    SubscriptionChanged,
    FeedUnsubscribed,
}

/// A stable crawl-domain event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrawlEventKind {
    TargetActivated,
    TargetPolicyChanged,
    TargetDeactivated,
    JobEnqueued,
    JobStarted,
}

/// A stable API contract event category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiEventKind {
    FeedSubscribed,
    FeedSubscribeRejected,
    FeedSubscriptionChanged,
    FeedUnsubscribed,
    FeedUnsubscribeRejected,
}

/// Event categories a worker is interested in consuming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInterests {
    kinds: Vec<EventKind>,
}

impl EventInterests {
    pub fn new(kinds: impl Into<Vec<EventKind>>) -> Self {
        Self {
            kinds: kinds.into(),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn kinds(&self) -> &[EventKind] {
        &self.kinds
    }

    pub fn contains(&self, kind: EventKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub fn matches_any(&self, kinds: &[EventKind]) -> bool {
        kinds.iter().any(|kind| self.contains(*kind))
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

/// A fact about the lifecycle of one accepted registry request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestEvent {
    SubscribeFeedRequested(SubscribeFeedRequested),
    SubscribeFeedRejected(SubscribeFeedRejected),
    UnsubscribeFeedRequested(UnsubscribeFeedRequested),
    UnsubscribeFeedRejected(UnsubscribeFeedRejected),
}

impl RequestEvent {
    pub fn kind(&self) -> RequestEventKind {
        match self {
            Self::SubscribeFeedRequested(_) => RequestEventKind::SubscribeFeedRequested,
            Self::SubscribeFeedRejected(_) => RequestEventKind::SubscribeFeedRejected,
            Self::UnsubscribeFeedRequested(_) => RequestEventKind::UnsubscribeFeedRequested,
            Self::UnsubscribeFeedRejected(_) => RequestEventKind::UnsubscribeFeedRejected,
        }
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
        Self::Request(RequestEvent::SubscribeFeedRequested(event))
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
        Self::Request(RequestEvent::SubscribeFeedRejected(event))
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
        Self::Request(RequestEvent::UnsubscribeFeedRequested(event))
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
        Self::Request(RequestEvent::UnsubscribeFeedRejected(event))
    }
}

/// A fact about subscription domain state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubEvent {
    FeedSubscribed(FeedSubscribedEvent),
    SubscriptionChanged(SubscriptionChangedEvent),
    FeedUnsubscribed(FeedUnsubscribedEvent),
}

impl SubEvent {
    pub fn kind(&self) -> SubEventKind {
        match self {
            Self::FeedSubscribed(_) => SubEventKind::FeedSubscribed,
            Self::SubscriptionChanged(_) => SubEventKind::SubscriptionChanged,
            Self::FeedUnsubscribed(_) => SubEventKind::FeedUnsubscribed,
        }
    }
}

/// A fact about crawl-domain state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlEvent {
    TargetActivated(CrawlTargetActivatedEvent),
    TargetPolicyChanged(CrawlTargetPolicyChangedEvent),
    TargetDeactivated(CrawlTargetDeactivatedEvent),
    JobEnqueued(CrawlJobEnqueuedEvent),
    JobStarted(CrawlJobStartedEvent),
}

impl CrawlEvent {
    pub fn kind(&self) -> CrawlEventKind {
        match self {
            Self::TargetActivated(_) => CrawlEventKind::TargetActivated,
            Self::TargetPolicyChanged(_) => CrawlEventKind::TargetPolicyChanged,
            Self::TargetDeactivated(_) => CrawlEventKind::TargetDeactivated,
            Self::JobEnqueued(_) => CrawlEventKind::JobEnqueued,
            Self::JobStarted(_) => CrawlEventKind::JobStarted,
        }
    }
}

/// Public event contract exposed through the API stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiEvent {
    FeedSubscribed(ApiFeedSubscribed),
    FeedSubscribeRejected(ApiFeedSubscribeRejected),
    FeedSubscriptionChanged(ApiFeedSubscriptionChanged),
    FeedUnsubscribed(ApiFeedUnsubscribed),
    FeedUnsubscribeRejected(ApiFeedUnsubscribeRejected),
}

impl ApiEvent {
    pub fn kind(&self) -> ApiEventKind {
        match self {
            Self::FeedSubscribed(_) => ApiEventKind::FeedSubscribed,
            Self::FeedSubscribeRejected(_) => ApiEventKind::FeedSubscribeRejected,
            Self::FeedSubscriptionChanged(_) => ApiEventKind::FeedSubscriptionChanged,
            Self::FeedUnsubscribed(_) => ApiEventKind::FeedUnsubscribed,
            Self::FeedUnsubscribeRejected(_) => ApiEventKind::FeedUnsubscribeRejected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedSubscribed {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl ApiFeedSubscribed {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedSubscribeRejected {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub reason: String,
}

impl ApiFeedSubscribeRejected {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedSubscriptionChanged {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl ApiFeedSubscriptionChanged {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedUnsubscribed {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl ApiFeedUnsubscribed {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedUnsubscribeRejected {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub reason: String,
}

impl ApiFeedUnsubscribeRejected {
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
    pub fn kind(&self) -> SubEventKind {
        match self {
            Self::Subscribed(_) => SubEventKind::FeedSubscribed,
            Self::Changed(_) => SubEventKind::SubscriptionChanged,
            Self::Unsubscribed(_) => SubEventKind::FeedUnsubscribed,
        }
    }

    pub fn affected_feed_url(&self) -> &FeedUrl {
        match self {
            Self::Subscribed(event) => &event.subscription.feed_url,
            Self::Changed(event) => &event.subscription.feed_url,
            Self::Unsubscribed(event) => &event.subscription.feed_url,
        }
    }
}

impl From<SubscriptionLifecycle> for SubEvent {
    fn from(event: SubscriptionLifecycle) -> Self {
        match event {
            SubscriptionLifecycle::Subscribed(event) => Self::FeedSubscribed(event),
            SubscriptionLifecycle::Changed(event) => Self::SubscriptionChanged(event),
            SubscriptionLifecycle::Unsubscribed(event) => Self::FeedUnsubscribed(event),
        }
    }
}

impl SubscriptionLifecycle {
    pub fn from_sub_event(event: SubEvent) -> Option<Self> {
        match event {
            SubEvent::FeedSubscribed(event) => Some(SubscriptionLifecycle::Subscribed(event)),
            SubEvent::SubscriptionChanged(event) => Some(SubscriptionLifecycle::Changed(event)),
            SubEvent::FeedUnsubscribed(event) => Some(SubscriptionLifecycle::Unsubscribed(event)),
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

impl From<FeedSubscribedEvent> for SubEvent {
    fn from(event: FeedSubscribedEvent) -> Self {
        Self::FeedSubscribed(event)
    }
}

impl From<FeedSubscribedEvent> for Event {
    fn from(event: FeedSubscribedEvent) -> Self {
        Self::Sub(event.into())
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

impl From<SubscriptionChangedEvent> for SubEvent {
    fn from(event: SubscriptionChangedEvent) -> Self {
        Self::SubscriptionChanged(event)
    }
}

impl From<SubscriptionChangedEvent> for Event {
    fn from(event: SubscriptionChangedEvent) -> Self {
        Self::Sub(event.into())
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

impl From<FeedUnsubscribedEvent> for SubEvent {
    fn from(event: FeedUnsubscribedEvent) -> Self {
        Self::FeedUnsubscribed(event)
    }
}

impl From<FeedUnsubscribedEvent> for Event {
    fn from(event: FeedUnsubscribedEvent) -> Self {
        Self::Sub(event.into())
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

impl From<CrawlTargetActivatedEvent> for CrawlEvent {
    fn from(event: CrawlTargetActivatedEvent) -> Self {
        Self::TargetActivated(event)
    }
}

impl From<CrawlTargetActivatedEvent> for Event {
    fn from(event: CrawlTargetActivatedEvent) -> Self {
        Self::Crawl(event.into())
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

impl From<CrawlTargetPolicyChangedEvent> for CrawlEvent {
    fn from(event: CrawlTargetPolicyChangedEvent) -> Self {
        Self::TargetPolicyChanged(event)
    }
}

impl From<CrawlTargetPolicyChangedEvent> for Event {
    fn from(event: CrawlTargetPolicyChangedEvent) -> Self {
        Self::Crawl(event.into())
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

impl From<CrawlTargetDeactivatedEvent> for CrawlEvent {
    fn from(event: CrawlTargetDeactivatedEvent) -> Self {
        Self::TargetDeactivated(event)
    }
}

impl From<CrawlTargetDeactivatedEvent> for Event {
    fn from(event: CrawlTargetDeactivatedEvent) -> Self {
        Self::Crawl(event.into())
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

impl From<CrawlJobEnqueuedEvent> for CrawlEvent {
    fn from(event: CrawlJobEnqueuedEvent) -> Self {
        Self::JobEnqueued(event)
    }
}

impl From<CrawlJobEnqueuedEvent> for Event {
    fn from(event: CrawlJobEnqueuedEvent) -> Self {
        Self::Crawl(event.into())
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

impl From<CrawlJobStartedEvent> for CrawlEvent {
    fn from(event: CrawlJobStartedEvent) -> Self {
        Self::JobStarted(event)
    }
}

impl From<CrawlJobStartedEvent> for Event {
    fn from(event: CrawlJobStartedEvent) -> Self {
        Self::Crawl(event.into())
    }
}
