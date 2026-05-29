use serde::{Deserialize, Serialize};

use crate::subscription::Subscription;

/// A domain fact produced by the registry after registry state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryEvent {
    /// A fact about a subscription relation between a subscriber and a feed.
    SubscriptionLifecycle(SubscriptionLifecycle),
}

impl RegistryEvent {
    pub fn kind(&self) -> RegistryEventKind {
        match self {
            Self::SubscriptionLifecycle(event) => event.kind(),
        }
    }
}

/// A stable event category used to route committed facts to interested consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryEventKind {
    FeedSubscribed,
    SubscriptionChanged,
    FeedUnsubscribed,
}

/// Event categories used by a journal read query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventReadFilter {
    kinds: &'static [RegistryEventKind],
}

impl EventReadFilter {
    pub const fn new(kinds: &'static [RegistryEventKind]) -> Self {
        Self { kinds }
    }

    pub const fn empty() -> Self {
        Self::new(&[])
    }

    pub fn kinds(self) -> &'static [RegistryEventKind] {
        self.kinds
    }

    pub fn contains(self, kind: RegistryEventKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub fn matches_any(self, kinds: &[RegistryEventKind]) -> bool {
        kinds.iter().any(|kind| self.contains(*kind))
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
    pub fn kind(&self) -> RegistryEventKind {
        match self {
            Self::Subscribed(_) => RegistryEventKind::FeedSubscribed,
            Self::Changed(_) => RegistryEventKind::SubscriptionChanged,
            Self::Unsubscribed(_) => RegistryEventKind::FeedUnsubscribed,
        }
    }
}

/// A subscription relation was created and became active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedSubscribed {
    /// The subscription relation that was created.
    pub subscription: Subscription,
}

impl FeedSubscribed {
    pub fn new(subscription: Subscription) -> Self {
        Self { subscription }
    }
}

/// An active subscription was updated without ending the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionChanged {
    /// The subscription relation whose registry-owned attributes changed.
    pub subscription: Subscription,
}

impl SubscriptionChanged {
    pub fn new(subscription: Subscription) -> Self {
        Self { subscription }
    }
}

/// A subscription relation was ended and is no longer active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedUnsubscribed {
    /// The subscription relation that ended.
    pub subscription: Subscription,
}

impl FeedUnsubscribed {
    pub fn new(subscription: Subscription) -> Self {
        Self { subscription }
    }
}
