use thiserror::Error;

use crate::{
    event::{FeedSubscribedEvent, FeedUnsubscribedEvent, SubEvent, SubscriptionChangedEvent},
    handler::Decider,
    subscription::{FeedSubscriptionAttrs, SubscriptionKey},
};

/// Current command-time state of one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubState {
    NotSubscribed,
    Subscribed,
}

/// Domain command over one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubCommand {
    Subscribe {
        subscription: SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
    },
    Unsubscribe {
        subscription: SubscriptionKey,
    },
}

impl SubCommand {
    pub fn subscription(&self) -> &SubscriptionKey {
        match self {
            Self::Subscribe { subscription, .. } | Self::Unsubscribe { subscription } => {
                subscription
            }
        }
    }
}

/// Domain rejection returned before any state mutation or journal append.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubReject {
    #[error("feed is not subscribed: {0:?}")]
    NotSubscribed(SubscriptionKey),
}

/// Decides subscription domain events from a subscription command and state.
#[derive(Debug, Clone, Copy, Default)]
pub struct SubDecider;

impl Decider for SubDecider {
    type Command = SubCommand;
    type State = SubState;
    type Event = SubEvent;
    type Reject = SubReject;

    fn decide(
        &self,
        command: Self::Command,
        state: Self::State,
    ) -> Result<Vec<Self::Event>, Self::Reject> {
        let event = match (command, state) {
            (
                SubCommand::Subscribe {
                    subscription,
                    attrs,
                },
                SubState::NotSubscribed,
            ) => SubEvent::Subscribed(FeedSubscribedEvent::new(subscription, attrs)),
            (
                SubCommand::Subscribe {
                    subscription,
                    attrs,
                },
                SubState::Subscribed,
            ) => SubEvent::Changed(SubscriptionChangedEvent::new(subscription, attrs)),
            (SubCommand::Unsubscribe { subscription }, SubState::Subscribed) => {
                SubEvent::Unsubscribed(FeedUnsubscribedEvent::new(subscription))
            }
            (SubCommand::Unsubscribe { subscription }, SubState::NotSubscribed) => {
                return Err(SubReject::NotSubscribed(subscription));
            }
        };
        Ok(vec![event])
    }
}

pub fn evolve(state: SubState, event: &SubEvent) -> SubState {
    match (state, event) {
        (_, SubEvent::Subscribed(_) | SubEvent::Changed(_)) => SubState::Subscribed,
        (_, SubEvent::Unsubscribed(_)) => SubState::NotSubscribed,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::handler::Decider;
    use crate::{
        SubscriberId,
        crawl::policy::{CrawlPolicy, PollingInterval},
    };

    #[test]
    fn decides_subscribe_when_not_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;
        let decider = SubDecider;

        let events = decider.decide(
            SubCommand::Subscribe {
                subscription: subscription.clone(),
                attrs: attrs.clone(),
            },
            SubState::NotSubscribed,
        )?;

        let [SubEvent::Subscribed(event)] = events.as_slice() else {
            panic!("expected Subscribed");
        };
        assert_eq!(event.subscription, subscription);
        assert_eq!(event.attrs, attrs);
        Ok(())
    }

    #[test]
    fn decides_change_when_already_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;
        let decider = SubDecider;

        let events = decider.decide(
            SubCommand::Subscribe {
                subscription: subscription.clone(),
                attrs: attrs.clone(),
            },
            SubState::Subscribed,
        )?;

        let [SubEvent::Changed(event)] = events.as_slice() else {
            panic!("expected Changed");
        };
        assert_eq!(event.subscription, subscription);
        assert_eq!(event.attrs, attrs);
        Ok(())
    }

    #[test]
    fn decides_unsubscribe_when_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let decider = SubDecider;

        let events = decider.decide(
            SubCommand::Unsubscribe {
                subscription: subscription.clone(),
            },
            SubState::Subscribed,
        )?;

        let [SubEvent::Unsubscribed(event)] = events.as_slice() else {
            panic!("expected Unsubscribed");
        };
        assert_eq!(event.subscription, subscription);
        Ok(())
    }

    #[test]
    fn rejects_unsubscribe_when_not_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let decider = SubDecider;

        let err = decider
            .decide(
                SubCommand::Unsubscribe {
                    subscription: subscription.clone(),
                },
                SubState::NotSubscribed,
            )
            .unwrap_err();

        assert_eq!(err, SubReject::NotSubscribed(subscription));
        Ok(())
    }

    #[test]
    fn evolves_subscription_state() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;
        let subscribed = SubEvent::Subscribed(FeedSubscribedEvent::new(
            subscription.clone(),
            attrs.clone(),
        ));
        let changed = SubEvent::Changed(SubscriptionChangedEvent::new(subscription.clone(), attrs));
        let unsubscribed = SubEvent::Unsubscribed(FeedUnsubscribedEvent::new(subscription));

        assert_eq!(
            evolve(SubState::NotSubscribed, &subscribed),
            SubState::Subscribed
        );
        assert_eq!(evolve(SubState::Subscribed, &changed), SubState::Subscribed);
        assert_eq!(
            evolve(SubState::Subscribed, &unsubscribed),
            SubState::NotSubscribed
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
