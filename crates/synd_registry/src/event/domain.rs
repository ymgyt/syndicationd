use std::fmt;

use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, Requirement};

use crate::{crawl::policy::CrawlPolicy, subscription::SubscriptionKey};

/// A typed fact recorded in the registry event journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Request(RequestEvent),
    Sub(SubEvent),
    Api(ApiEvent),
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::Request(event) => event.kind().into(),
            Self::Sub(event) => event.kind().into(),
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

/// A fact about subscription domain state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubEvent {
    FeedSubscribed(FeedSubscribed),
    SubscriptionChanged(SubscriptionChanged),
    FeedUnsubscribed(FeedUnsubscribed),
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
    Subscribed(FeedSubscribed),
    /// An active subscription changed its registry-owned attributes.
    Changed(SubscriptionChanged),
    /// The subscriber stopped subscribing to the feed.
    Unsubscribed(FeedUnsubscribed),
}

impl SubscriptionLifecycle {
    pub fn kind(&self) -> SubEventKind {
        match self {
            Self::Subscribed(_) => SubEventKind::FeedSubscribed,
            Self::Changed(_) => SubEventKind::SubscriptionChanged,
            Self::Unsubscribed(_) => SubEventKind::FeedUnsubscribed,
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
pub struct FeedSubscribed {
    /// The subscription relation that was created.
    pub subscription: SubscriptionKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

impl FeedSubscribed {
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

/// An active subscription was updated without ending the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionChanged {
    /// The subscription relation whose registry-owned attributes changed.
    pub subscription: SubscriptionKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

impl SubscriptionChanged {
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

/// A subscription relation was ended and is no longer active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedUnsubscribed {
    /// The subscription relation that ended.
    pub subscription: SubscriptionKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

impl FeedUnsubscribed {
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
