use thiserror::Error;

use crate::{
    event::{FeedSubscribedEvent, FeedUnsubscribedEvent, SubEvent, SubscriptionChangedEvent},
    subscription::{FeedSubscriptionAttrs, SubscribeOutcome, SubscriptionKey, UnsubscribeOutcome},
};

/// Current command-time state of one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubState {
    NotSubscribed,
    Subscribed,
}

/// Domain rejection returned before any state mutation or journal append.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubReject {
    #[error("feed is not subscribed: {0:?}")]
    NotSubscribed(SubscriptionKey),
}

impl SubState {
    pub(super) fn subscribe(
        self,
        subscription: SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
    ) -> (SubscribeOutcome, SubEvent) {
        match self {
            Self::NotSubscribed => (
                SubscribeOutcome::Subscribed(subscription.clone()),
                SubEvent::Subscribed(FeedSubscribedEvent::new(subscription, attrs)),
            ),
            Self::Subscribed => (
                SubscribeOutcome::Changed(subscription.clone()),
                SubEvent::Changed(SubscriptionChangedEvent::new(subscription, attrs)),
            ),
        }
    }

    pub(super) fn unsubscribe(
        self,
        subscription: SubscriptionKey,
    ) -> Result<(UnsubscribeOutcome, SubEvent), SubReject> {
        match self {
            Self::Subscribed => Ok((
                UnsubscribeOutcome::Unsubscribed(subscription.clone()),
                SubEvent::Unsubscribed(FeedUnsubscribedEvent::new(subscription)),
            )),
            Self::NotSubscribed => Err(SubReject::NotSubscribed(subscription)),
        }
    }

    /// The state after one subscription event.
    #[must_use]
    pub fn evolve(self, event: &SubEvent) -> Self {
        match (self, event) {
            (_, SubEvent::Subscribed(_) | SubEvent::Changed(_)) => Self::Subscribed,
            (_, SubEvent::Unsubscribed(_)) => Self::NotSubscribed,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_feed::types::{FeedUrl, SubscriptionLabels};

    use super::*;
    use crate::{
        SubscriberId,
        crawl::policy::{CrawlPolicy, PollingInterval},
    };

    #[test]
    fn decides_subscribe_when_not_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let attrs = attrs()?;
        let (_, event) = SubState::NotSubscribed.subscribe(subscription.clone(), attrs.clone());

        let SubEvent::Subscribed(event) = event else {
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
        let (_, event) = SubState::Subscribed.subscribe(subscription.clone(), attrs.clone());

        let SubEvent::Changed(event) = event else {
            panic!("expected Changed");
        };
        assert_eq!(event.subscription, subscription);
        assert_eq!(event.attrs, attrs);
        Ok(())
    }

    #[test]
    fn decides_unsubscribe_when_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let (_, event) = SubState::Subscribed.unsubscribe(subscription.clone())?;

        let SubEvent::Unsubscribed(event) = event else {
            panic!("expected Unsubscribed");
        };
        assert_eq!(event.subscription, subscription);
        Ok(())
    }

    #[test]
    fn rejects_unsubscribe_when_not_subscribed() -> anyhow::Result<()> {
        let subscription = subscription()?;
        let err = SubState::NotSubscribed
            .unsubscribe(subscription.clone())
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
            SubState::NotSubscribed.evolve(&subscribed),
            SubState::Subscribed
        );
        assert_eq!(SubState::Subscribed.evolve(&changed), SubState::Subscribed);
        assert_eq!(
            SubState::Subscribed.evolve(&unsubscribed),
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
            labels: SubscriptionLabels::default(),
            crawl_policy: CrawlPolicy::interval(PollingInterval::try_from(Duration::from_hours(
                1,
            ))?),
        })
    }
}
