use std::num::NonZeroUsize;

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::policy::{CrawlPolicy, PollingInterval, PollingPolicy},
    db::{FeedRegistryDb, RegistryTx},
    event::{
        ConsumeContext, Consumer, Event, EventInterests, Processor, ProcessorError, ProcessorId,
        ProcessorResult, SubEvent, SubEventKind, SubscriptionLifecycle, Transactional,
    },
    subscription::SubscriptionKey,
};

/// Current crawl target state derived from active subscriptions for one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTarget {
    pub feed_url: FeedUrl,
    pub state: CrawlTargetState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CrawlTarget {
    pub fn new(feed_url: FeedUrl, state: CrawlTargetState, now: DateTime<Utc>) -> Self {
        Self {
            feed_url,
            state,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn inactive(feed_url: FeedUrl, now: DateTime<Utc>) -> Self {
        Self::new(feed_url, CrawlTargetState::Inactive, now)
    }
}

/// Current state of one feed endpoint inside the crawl target list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlTargetState {
    Active {
        subscription_count: NonZeroUsize,
        effective_policy: CrawlPolicy,
    },
    Inactive,
}

/// Current `feed_endpoint_subscription` row data used to derive a crawl target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedEndpointSubscription {
    pub subscription: SubscriptionKey,
    pub crawl_policy: CrawlPolicy,
}

impl FeedEndpointSubscription {
    pub fn new(subscription: SubscriptionKey, crawl_policy: CrawlPolicy) -> Self {
        Self {
            subscription,
            crawl_policy,
        }
    }
}

/// Current subscription relations for one feed endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedEndpointSubscriptionSet {
    pub feed_url: FeedUrl,
    pub subscriptions: Vec<FeedEndpointSubscription>,
}

impl FeedEndpointSubscriptionSet {
    pub fn new(feed_url: FeedUrl, subscriptions: Vec<FeedEndpointSubscription>) -> Self {
        debug_assert!(
            subscriptions
                .iter()
                .all(|subscription| subscription.subscription.feed_url == feed_url)
        );
        Self {
            feed_url,
            subscriptions,
        }
    }

    pub fn crawl_target_decision(self) -> CrawlTargetDecision {
        let state = if let Some(subscription_count) = NonZeroUsize::new(self.subscriptions.len()) {
            CrawlTargetState::Active {
                subscription_count,
                effective_policy: effective_policy(&self.subscriptions),
            }
        } else {
            CrawlTargetState::Inactive
        };

        CrawlTargetDecision {
            feed_url: self.feed_url,
            state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTargetDecision {
    feed_url: FeedUrl,
    state: CrawlTargetState,
}

impl CrawlTargetDecision {
    pub fn into_target(self, now: DateTime<Utc>) -> CrawlTarget {
        CrawlTarget::new(self.feed_url, self.state, now)
    }
}

/// Subscription lifecycle events relevant to the crawl target list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTargetListInput {
    event: SubscriptionLifecycle,
}

impl CrawlTargetListInput {
    pub fn new(event: SubscriptionLifecycle) -> Self {
        Self { event }
    }
}

impl TryFrom<Event> for CrawlTargetListInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Sub(SubEvent::FeedSubscribed(event)) => {
                Ok(Self::new(SubscriptionLifecycle::Subscribed(event)))
            }
            Event::Sub(SubEvent::SubscriptionChanged(event)) => {
                Ok(Self::new(SubscriptionLifecycle::Changed(event)))
            }
            Event::Sub(SubEvent::FeedUnsubscribed(event)) => {
                Ok(Self::new(SubscriptionLifecycle::Unsubscribed(event)))
            }
            event => Err(ProcessorError::UnexpectedEvent {
                expected: "crawl target list event",
                actual: event.kind(),
            }),
        }
    }
}

/// Consumer that reacts to subscription events for the crawl target list.
#[derive(Debug, Clone)]
pub struct CrawlTargetListProj;

impl CrawlTargetListProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CrawlTargetListProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CrawlTargetListProj {
    type Input = CrawlTargetListInput;
    type Phase = Transactional;

    fn id(&self) -> ProcessorId {
        ProcessorId::CrawlTargetProjection
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            SubEventKind::FeedSubscribed.into(),
            SubEventKind::SubscriptionChanged.into(),
            SubEventKind::FeedUnsubscribed.into(),
        ])
    }
}

impl<S> Consumer<S> for CrawlTargetListProj
where
    S: FeedRegistryDb,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<()> {
        let Self::Input { event } = input;

        let feed_url = event.affected_feed_url().clone();
        let subscriptions = cx.load_feed_endpoint_subscriptions(&feed_url).await?;
        let now = Utc::now();
        let target = subscriptions.crawl_target_decision().into_target(now);
        cx.upsert_crawl_target(&target).await?;

        debug!(
            feed_url = target.feed_url.as_str(),
            ?target.state,
            "crawl target reconciled"
        );
        Ok(())
    }
}

fn effective_policy(subscriptions: &[FeedEndpointSubscription]) -> CrawlPolicy {
    let interval = subscriptions
        .iter()
        .filter_map(|subscription| match subscription.crawl_policy.polling {
            PollingPolicy::Manual => None,
            PollingPolicy::Interval { interval } => Some(interval),
        })
        .reduce(PollingInterval::min);

    match interval {
        Some(interval) => CrawlPolicy::interval(interval),
        None => CrawlPolicy::manual(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::subscription::{SubscriberId, SubscriptionKey};

    fn interval(duration: Duration) -> PollingInterval {
        PollingInterval::try_from(duration).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn subscription(
        subscriber_id: &'static str,
        crawl_policy: CrawlPolicy,
    ) -> FeedEndpointSubscription {
        FeedEndpointSubscription::new(
            SubscriptionKey::new(SubscriberId::new(subscriber_id), feed_url()),
            crawl_policy,
        )
    }

    fn subscription_set(
        subscriptions: Vec<FeedEndpointSubscription>,
    ) -> FeedEndpointSubscriptionSet {
        FeedEndpointSubscriptionSet::new(feed_url(), subscriptions)
    }

    #[test]
    fn crawl_target_decision_uses_shortest_interval_subscription() {
        let subscriptions = subscription_set(vec![
            subscription(
                "one-hour",
                CrawlPolicy::interval(interval(Duration::from_hours(1))),
            ),
            subscription(
                "ten-minutes",
                CrawlPolicy::interval(interval(Duration::from_mins(10))),
            ),
            subscription("manual", CrawlPolicy::manual()),
        ]);

        let target = subscriptions
            .crawl_target_decision()
            .into_target(Utc::now());
        let CrawlTargetState::Active {
            subscription_count,
            effective_policy,
        } = target.state
        else {
            panic!("target should be active");
        };

        assert_eq!(subscription_count.get(), 3);
        assert_eq!(
            effective_policy.polling,
            PollingPolicy::Interval {
                interval: interval(Duration::from_mins(10))
            }
        );
    }

    #[test]
    fn crawl_target_decision_is_manual_when_all_subscriptions_are_manual() {
        let subscriptions = subscription_set(vec![subscription("manual", CrawlPolicy::manual())]);

        let target = subscriptions
            .crawl_target_decision()
            .into_target(Utc::now());

        assert_eq!(
            target.state,
            CrawlTargetState::Active {
                subscription_count: NonZeroUsize::new(1).unwrap(),
                effective_policy: CrawlPolicy::manual(),
            }
        );
    }

    #[test]
    fn crawl_target_decision_is_inactive_without_subscriptions() {
        let subscriptions = subscription_set(Vec::new());

        let target = subscriptions
            .crawl_target_decision()
            .into_target(Utc::now());

        assert_eq!(target.state, CrawlTargetState::Inactive);
    }
}
