use thiserror::Error;

use crate::{
    event::{Event, FeedSubscribedEvent, FeedUnsubscribedEvent, SubscriptionChangedEvent},
    subscription::{FeedSubscriptionAttrs, SubscriptionKey},
};

pub trait Decider {
    type Command;
    type State;
    type Event;
    type Reject;

    fn decide(
        command: Self::Command,
        state: Self::State,
        subscription: SubscriptionKey,
    ) -> Result<Self::Event, Self::Reject>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SubscriptionDecider;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionState {
    NotSubscribed,
    Subscribed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionCommand {
    Subscribe { attrs: FeedSubscriptionAttrs },
    Unsubscribe,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubscriptionReject {
    #[error("feed is not subscribed: {0:?}")]
    NotSubscribed(SubscriptionKey),
}

pub fn decide(
    command: SubscriptionCommand,
    state: SubscriptionState,
    subscription: SubscriptionKey,
) -> Result<Event, SubscriptionReject> {
    Ok(match (command, state) {
        (SubscriptionCommand::Subscribe { attrs }, SubscriptionState::NotSubscribed) => {
            FeedSubscribedEvent::new(subscription, attrs).into()
        }
        (SubscriptionCommand::Subscribe { attrs }, SubscriptionState::Subscribed) => {
            SubscriptionChangedEvent::new(subscription, attrs).into()
        }
        (SubscriptionCommand::Unsubscribe, SubscriptionState::Subscribed) => {
            FeedUnsubscribedEvent::new(subscription).into()
        }
        (SubscriptionCommand::Unsubscribe, SubscriptionState::NotSubscribed) => {
            return Err(SubscriptionReject::NotSubscribed(subscription));
        }
    })
}

impl Decider for SubscriptionDecider {
    type Command = SubscriptionCommand;
    type State = SubscriptionState;
    type Event = Event;
    type Reject = SubscriptionReject;

    fn decide(
        command: Self::Command,
        state: Self::State,
        subscription: SubscriptionKey,
    ) -> Result<Self::Event, Self::Reject> {
        decide(command, state, subscription)
    }
}

pub fn evolve(state: SubscriptionState, event: &Event) -> SubscriptionState {
    match event {
        Event::FeedSubscribed(_) | Event::SubscriptionChanged(_) => SubscriptionState::Subscribed,
        Event::FeedUnsubscribed(_) => SubscriptionState::NotSubscribed,
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::{
        SubscriberId,
        crawl::policy::{CrawlPolicy, PollingInterval},
    };

    #[test]
    fn decides_subscribe_when_not_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;

        let event = decide(
            SubscriptionCommand::Subscribe {
                attrs: attrs.clone(),
            },
            SubscriptionState::NotSubscribed,
            subscription.clone(),
        )?;

        let Event::FeedSubscribed(event) = event else {
            panic!("expected FeedSubscribed");
        };
        assert_eq!(event.subscription, subscription);
        assert_eq!(event.attrs, attrs);
        Ok(())
    }

    #[test]
    fn decides_change_when_already_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;

        let event = decide(
            SubscriptionCommand::Subscribe {
                attrs: attrs.clone(),
            },
            SubscriptionState::Subscribed,
            subscription.clone(),
        )?;

        let Event::SubscriptionChanged(event) = event else {
            panic!("expected SubscriptionChanged");
        };
        assert_eq!(event.subscription, subscription);
        assert_eq!(event.attrs, attrs);
        Ok(())
    }

    #[test]
    fn decides_unsubscribe_when_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;

        let event = decide(
            SubscriptionCommand::Unsubscribe,
            SubscriptionState::Subscribed,
            subscription.clone(),
        )?;

        let Event::FeedUnsubscribed(event) = event else {
            panic!("expected FeedUnsubscribed");
        };
        assert_eq!(event.subscription, subscription);
        Ok(())
    }

    #[test]
    fn rejects_unsubscribe_when_not_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;

        let err = decide(
            SubscriptionCommand::Unsubscribe,
            SubscriptionState::NotSubscribed,
            subscription.clone(),
        )
        .unwrap_err();

        assert_eq!(err, SubscriptionReject::NotSubscribed(subscription));
        Ok(())
    }

    #[test]
    fn evolves_subscription_state() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;
        let subscribed = FeedSubscribedEvent::new(subscription.clone(), attrs.clone()).into();
        let changed = SubscriptionChangedEvent::new(subscription.clone(), attrs).into();
        let unsubscribed = FeedUnsubscribedEvent::new(subscription).into();

        assert_eq!(
            evolve(SubscriptionState::NotSubscribed, &subscribed),
            SubscriptionState::Subscribed
        );
        assert_eq!(
            evolve(SubscriptionState::Subscribed, &changed),
            SubscriptionState::Subscribed
        );
        assert_eq!(
            evolve(SubscriptionState::Subscribed, &unsubscribed),
            SubscriptionState::NotSubscribed
        );
        Ok(())
    }

    fn subscription() -> anyhow::Result<SubscriptionKey> {
        Ok(SubscriptionKey::new(
            SubscriberId::new("local"),
            FeedUrl::parse("https://example.com/feed.xml")?,
        ))
    }

    fn attrs() -> anyhow::Result<FeedSubscriptionAttrs> {
        Ok(FeedSubscriptionAttrs {
            requirement: None,
            category: None,
            crawl_policy: CrawlPolicy::interval(PollingInterval::try_from(Duration::from_hours(
                1,
            ))?),
        })
    }
}
