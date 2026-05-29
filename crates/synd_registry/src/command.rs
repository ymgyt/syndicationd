use crate::{
    event::{FeedSubscribed, FeedUnsubscribed, RegistryEvent, SubscriptionLifecycle},
    subscription::Subscription,
};

/// A request to change registry state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryCommand {
    Subscribe(Subscribe),
    Unsubscribe(Unsubscribe),
}

impl RegistryCommand {
    pub fn into_events(self) -> Vec<RegistryEvent> {
        match self {
            Self::Subscribe(command) => vec![command.into_event()],
            Self::Unsubscribe(command) => vec![command.into_event()],
        }
    }
}

/// Request to start a subscription relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscribe {
    pub subscription: Subscription,
}

impl Subscribe {
    pub fn into_event(self) -> RegistryEvent {
        RegistryEvent::SubscriptionLifecycle(SubscriptionLifecycle::Subscribed(
            FeedSubscribed::new(self.subscription),
        ))
    }
}

/// Request to end a subscription relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsubscribe {
    pub subscription: Subscription,
}

impl Unsubscribe {
    pub fn into_event(self) -> RegistryEvent {
        RegistryEvent::SubscriptionLifecycle(SubscriptionLifecycle::Unsubscribed(
            FeedUnsubscribed::new(self.subscription),
        ))
    }
}
