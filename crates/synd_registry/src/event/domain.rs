use chrono::{DateTime, Utc};
use derive_more::From;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use strum::{Display, EnumDiscriminants, EnumString, IntoStaticStr};
use synd_feed::{entry::EntryId, types::FeedUrl};

use crate::{
    crawl::{blob::BlobRef, job::CrawlJobId, policy::CrawlPolicy},
    subscription::{FeedSubscriptionAttrs, SubscriberId, SubscriptionKey},
};

/// A typed fact recorded in the registry event journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, From, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum_discriminants(name(EventType))]
#[strum_discriminants(derive(Hash, Display, EnumString, IntoStaticStr))]
pub enum Event {
    #[serde(rename = "sub.feed.subscribed")]
    #[strum_discriminants(strum(serialize = "sub.feed.subscribed"))]
    FeedSubscribed(FeedSubscribedEvent),
    #[serde(rename = "sub.subscription.changed")]
    #[strum_discriminants(strum(serialize = "sub.subscription.changed"))]
    SubscriptionChanged(SubscriptionChangedEvent),
    #[serde(rename = "sub.feed.unsubscribed")]
    #[strum_discriminants(strum(serialize = "sub.feed.unsubscribed"))]
    FeedUnsubscribed(FeedUnsubscribedEvent),
    #[serde(rename = "crawl.target.activated")]
    #[strum_discriminants(strum(serialize = "crawl.target.activated"))]
    CrawlTargetActivated(CrawlTargetActivatedEvent),
    #[serde(rename = "crawl.target.policy_changed")]
    #[strum_discriminants(strum(serialize = "crawl.target.policy_changed"))]
    CrawlTargetPolicyChanged(CrawlTargetPolicyChangedEvent),
    #[serde(rename = "crawl.target.deactivated")]
    #[strum_discriminants(strum(serialize = "crawl.target.deactivated"))]
    CrawlTargetDeactivated(CrawlTargetDeactivatedEvent),
    #[serde(rename = "crawl.requested")]
    #[strum_discriminants(strum(serialize = "crawl.requested"))]
    CrawlRequested(CrawlRequestedEvent),
    #[serde(rename = "crawl.job.finished")]
    #[strum_discriminants(strum(serialize = "crawl.job.finished"))]
    CrawlJobFinished(CrawlJobFinishedEvent),
    #[serde(rename = "entry.discovered")]
    #[strum_discriminants(strum(serialize = "entry.discovered"))]
    EntryDiscovered(EntryDiscoveredEvent),
    #[serde(rename = "entry.changed")]
    #[strum_discriminants(strum(serialize = "entry.changed"))]
    EntryChanged(EntryChangedEvent),
    #[serde(rename = "timeline.changed")]
    #[strum_discriminants(strum(serialize = "timeline.changed"))]
    TimelineChanged(TimelineChangedEvent),
}

impl Event {
    pub fn event_type(&self) -> EventType {
        EventType::from(self)
    }
}

/// Event payload persisted in the registry journal.
pub trait RegistryEvent: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    const TYPE: EventType;
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

/// A domain fact about the lifecycle of one feed subscription relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubEvent {
    /// The subscriber started subscribing to the feed.
    Subscribed(FeedSubscribedEvent),
    /// An active subscription changed its registry-owned attributes.
    Changed(SubscriptionChangedEvent),
    /// The subscriber stopped subscribing to the feed.
    Unsubscribed(FeedUnsubscribedEvent),
}

impl SubEvent {
    pub fn subscription(&self) -> &SubscriptionKey {
        match self {
            Self::Subscribed(event) => &event.subscription,
            Self::Changed(event) => &event.subscription,
            Self::Unsubscribed(event) => &event.subscription,
        }
    }

    pub fn affected_feed_url(&self) -> &FeedUrl {
        match self {
            Self::Subscribed(event) => &event.subscription.feed_url,
            Self::Changed(event) => &event.subscription.feed_url,
            Self::Unsubscribed(event) => &event.subscription.feed_url,
        }
    }

    pub fn event_type(&self) -> EventType {
        match self {
            Self::Subscribed(_) => FeedSubscribedEvent::TYPE,
            Self::Changed(_) => SubscriptionChangedEvent::TYPE,
            Self::Unsubscribed(_) => FeedUnsubscribedEvent::TYPE,
        }
    }

    pub fn outcome_label(&self) -> &'static str {
        match self {
            Self::Subscribed(_) => "subscribed",
            Self::Changed(_) => "changed",
            Self::Unsubscribed(_) => "unsubscribed",
        }
    }
}

impl From<SubEvent> for Event {
    fn from(event: SubEvent) -> Self {
        match event {
            SubEvent::Subscribed(event) => Self::FeedSubscribed(event),
            SubEvent::Changed(event) => Self::SubscriptionChanged(event),
            SubEvent::Unsubscribed(event) => Self::FeedUnsubscribed(event),
        }
    }
}

/// A subscription relation was created and became active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedSubscribedEvent {
    /// The subscription relation that was created.
    pub subscription: SubscriptionKey,
    pub attrs: FeedSubscriptionAttrs,
}

impl FeedSubscribedEvent {
    pub fn new(subscription: SubscriptionKey, attrs: FeedSubscriptionAttrs) -> Self {
        Self {
            subscription,
            attrs,
        }
    }
}

/// An active subscription was updated without ending the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionChangedEvent {
    /// The subscription relation whose registry-owned attributes changed.
    pub subscription: SubscriptionKey,
    pub attrs: FeedSubscriptionAttrs,
}

impl SubscriptionChangedEvent {
    pub fn new(subscription: SubscriptionKey, attrs: FeedSubscriptionAttrs) -> Self {
        Self {
            subscription,
            attrs,
        }
    }
}

/// A subscription relation was ended and is no longer active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedUnsubscribedEvent {
    /// The subscription relation that ended.
    pub subscription: SubscriptionKey,
}

impl FeedUnsubscribedEvent {
    pub fn new(subscription: SubscriptionKey) -> Self {
        Self { subscription }
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

/// A crawl was explicitly requested for one feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlRequestedEvent {
    pub feed_url: FeedUrl,
}

impl CrawlRequestedEvent {
    pub fn new(feed_url: FeedUrl) -> Self {
        Self { feed_url }
    }
}

/// A crawl job completed and moved out of the running set.
///
/// `body_blob` references the fetched body when the crawl observed one;
/// events carry blob references, never the bytes themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlJobFinishedEvent {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
    pub started_at: DateTime<Utc>,
    pub body_blob: Option<BlobRef>,
}

impl CrawlJobFinishedEvent {
    pub fn new(
        job_id: CrawlJobId,
        feed_url: FeedUrl,
        started_at: DateTime<Utc>,
        body_blob: Option<BlobRef>,
    ) -> Self {
        Self {
            job_id,
            feed_url,
            started_at,
            body_blob,
        }
    }
}

/// An entry became known to the registry for the first time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryDiscoveredEvent {
    pub feed_url: FeedUrl,
    pub entry_id: EntryId,
}

impl EntryDiscoveredEvent {
    /// Creates an event for an entry first observed by the registry.
    pub fn new(feed_url: FeedUrl, entry_id: EntryId) -> Self {
        Self { feed_url, entry_id }
    }
}

/// The current state of a known entry changed after a feed source was applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryChangedEvent {
    pub feed_url: FeedUrl,
    pub entry_id: EntryId,
}

impl EntryChangedEvent {
    /// Creates an event for a changed entry observed by a feed source.
    pub fn new(feed_url: FeedUrl, entry_id: EntryId) -> Self {
        Self { feed_url, entry_id }
    }
}

/// Timeline membership changed for one subscriber's timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineChangedEvent {
    pub subscriber_id: SubscriberId,
    pub affected_feeds: Vec<FeedUrl>,
}

impl TimelineChangedEvent {
    pub fn new(subscriber_id: SubscriberId, affected_feeds: Vec<FeedUrl>) -> Self {
        Self {
            subscriber_id,
            affected_feeds,
        }
    }
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

impl RegistryEvent for CrawlRequestedEvent {
    const TYPE: EventType = EventType::CrawlRequested;
}

impl RegistryEvent for CrawlJobFinishedEvent {
    const TYPE: EventType = EventType::CrawlJobFinished;
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
