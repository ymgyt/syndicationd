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
    subscription::Subscription,
};

/// Current crawl target state derived from active subscriptions for one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTarget {
    pub feed_url: FeedUrl,
    pub is_active: bool,
    pub subscription_count: usize,
    pub crawl_policy: Option<CrawlPolicy>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CrawlTarget {
    pub fn active(
        feed_url: FeedUrl,
        subscription_count: usize,
        crawl_policy: CrawlPolicy,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            is_active: true,
            subscription_count,
            crawl_policy: Some(crawl_policy),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn inactive(feed_url: FeedUrl, now: DateTime<Utc>) -> Self {
        Self {
            feed_url,
            is_active: false,
            subscription_count: 0,
            crawl_policy: None,
            created_at: now,
            updated_at: now,
        }
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

    pub fn into_event(self) -> SubscriptionLifecycle {
        self.event
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
        let feed_url = affected_feed_url(input.into_event());
        let subscriptions = cx.list_active_subscriptions_for_endpoint(&feed_url).await?;
        let subscription_count = subscriptions.len();
        let now = Utc::now();
        let target = if let Some(policy) = CrawlPolicyResolver::resolve(&subscriptions) {
            CrawlTarget::active(feed_url.clone(), subscription_count, policy, now)
        } else {
            CrawlTarget::inactive(feed_url.clone(), now)
        };
        cx.upsert_crawl_target(target).await?;

        debug!(
            feed_url = feed_url.as_str(),
            "crawl target list projector reconciled feed"
        );
        Ok(())
    }
}

fn affected_feed_url(event: SubscriptionLifecycle) -> FeedUrl {
    match event {
        SubscriptionLifecycle::Subscribed(event) => event.subscription.feed_url,
        SubscriptionLifecycle::Changed(event) => event.subscription.feed_url,
        SubscriptionLifecycle::Unsubscribed(event) => event.subscription.feed_url,
    }
}

struct CrawlPolicyResolver;

impl CrawlPolicyResolver {
    fn resolve(subscriptions: &[Subscription]) -> Option<CrawlPolicy> {
        if subscriptions.is_empty() {
            return None;
        }

        let interval = subscriptions
            .iter()
            .filter_map(|subscription| match subscription.crawl_policy.polling {
                PollingPolicy::Manual => None,
                PollingPolicy::Interval { interval } => Some(interval),
            })
            .reduce(PollingInterval::min);

        Some(match interval {
            Some(interval) => CrawlPolicy::interval(interval),
            None => CrawlPolicy::manual(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::subscription::SubscriberId;

    fn interval(duration: Duration) -> PollingInterval {
        PollingInterval::try_from(duration).unwrap()
    }

    fn subscription(crawl_policy: CrawlPolicy) -> Subscription {
        let now = Utc::now();
        Subscription {
            subscriber_id: SubscriberId::new("local"),
            feed_url: FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            requirement: None,
            category: None,
            crawl_policy,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn crawl_policy_resolver_uses_shortest_interval_subscription() {
        let subscriptions = [
            subscription(CrawlPolicy::interval(interval(Duration::from_hours(1)))),
            subscription(CrawlPolicy::interval(interval(Duration::from_mins(10)))),
            subscription(CrawlPolicy::manual()),
        ];

        let policy = CrawlPolicyResolver::resolve(&subscriptions).unwrap();

        assert_eq!(
            policy.polling,
            PollingPolicy::Interval {
                interval: interval(Duration::from_mins(10))
            }
        );
    }

    #[test]
    fn crawl_policy_resolver_is_manual_when_all_subscriptions_are_manual() {
        let subscriptions = [subscription(CrawlPolicy::manual())];

        let policy = CrawlPolicyResolver::resolve(&subscriptions).unwrap();

        assert_eq!(policy, CrawlPolicy::manual());
    }

    #[test]
    fn crawl_policy_resolver_returns_none_without_subscriptions() {
        assert_eq!(CrawlPolicyResolver::resolve(&[]), None);
    }
}
