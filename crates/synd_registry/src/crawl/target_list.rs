use std::num::NonZeroUsize;

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::{debug, info};

use crate::{
    crawl::policy::{CrawlPolicy, PollingInterval, PollingPolicy},
    db::{CrawlTargetStore, FeedRegistryDb, SubscriptionStore},
    event::{
        CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent,
        Event, EventInput, EventType, FeedSubscribedEvent, FeedUnsubscribedEvent, Processor,
        ProcessorError, ProcessorId, ProcessorResult, Projector, RegistryEvent, SubEvent,
        SubscriptionChangedEvent,
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

    /// The lifecycle event represented by the transition from `previous` to
    /// this target, if any.
    pub fn lifecycle_event(&self, previous: Option<&CrawlTarget>) -> Option<Event> {
        match (previous.map(|target| &target.state), &self.state) {
            (
                None | Some(CrawlTargetState::Inactive),
                CrawlTargetState::Active {
                    effective_policy, ..
                },
            ) => Some(
                CrawlTargetActivatedEvent::new(self.feed_url.clone(), *effective_policy).into(),
            ),
            (
                Some(CrawlTargetState::Active {
                    effective_policy: previous_policy,
                    ..
                }),
                CrawlTargetState::Active {
                    effective_policy, ..
                },
            ) if previous_policy != effective_policy => Some(
                CrawlTargetPolicyChangedEvent::new(self.feed_url.clone(), *effective_policy).into(),
            ),
            (Some(CrawlTargetState::Active { .. }), CrawlTargetState::Inactive) => {
                Some(CrawlTargetDeactivatedEvent::new(self.feed_url.clone()).into())
            }
            _ => None,
        }
    }

    fn active_subscription_count(&self) -> usize {
        match &self.state {
            CrawlTargetState::Active {
                subscription_count, ..
            } => subscription_count.get(),
            CrawlTargetState::Inactive => 0,
        }
    }

    fn log_lifecycle_event(&self, event: &Event) {
        match event {
            Event::CrawlTargetActivated(event) => {
                info!(
                    feed_url = event.feed_url.as_str(),
                    subscriptions = self.active_subscription_count(),
                    policy = %event.policy.polling,
                    "crawl target activated"
                );
            }
            Event::CrawlTargetPolicyChanged(event) => {
                info!(
                    feed_url = event.feed_url.as_str(),
                    subscriptions = self.active_subscription_count(),
                    policy = %event.policy.polling,
                    "crawl target policy changed"
                );
            }
            Event::CrawlTargetDeactivated(event) => {
                info!(
                    feed_url = event.feed_url.as_str(),
                    "crawl target deactivated"
                );
            }
            _ => {}
        }
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
                effective_policy: self.effective_policy(),
            }
        } else {
            CrawlTargetState::Inactive
        };

        CrawlTargetDecision {
            feed_url: self.feed_url,
            state,
        }
    }

    /// The most demanding polling policy across all subscriptions: the
    /// shortest interval wins; manual applies only when every subscription is
    /// manual.
    fn effective_policy(&self) -> CrawlPolicy {
        let interval = self
            .subscriptions
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
}

/// Desired crawl target state derived from current endpoint subscriptions.
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

/// Event input used to project crawl target state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTargetProjInput {
    event: SubEvent,
    occurred_at: DateTime<Utc>,
}

impl CrawlTargetProjInput {
    pub fn new(event: SubEvent, occurred_at: DateTime<Utc>) -> Self {
        Self { event, occurred_at }
    }
}

impl EventInput for CrawlTargetProjInput {
    const INTERESTS: &'static [EventType] = &[
        FeedSubscribedEvent::TYPE,
        SubscriptionChangedEvent::TYPE,
        FeedUnsubscribedEvent::TYPE,
    ];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::FeedSubscribed(event) => Ok(Self::new(SubEvent::Subscribed(event), occurred_at)),
            Event::SubscriptionChanged(event) => {
                Ok(Self::new(SubEvent::Changed(event), occurred_at))
            }
            Event::FeedUnsubscribed(event) => {
                Ok(Self::new(SubEvent::Unsubscribed(event), occurred_at))
            }
            event => Err(ProcessorError::unexpected_input(
                "crawl target projection event",
                &event,
            )),
        }
    }
}

/// Projects subscription lifecycle events into the crawl target list.
#[derive(Debug, Clone)]
pub struct CrawlTargetProj;

impl CrawlTargetProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CrawlTargetProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CrawlTargetProj {
    type Input = CrawlTargetProjInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::CrawlTargetProjection
    }
}

impl<S> Projector<S> for CrawlTargetProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlTargetStore + SubscriptionStore + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let Self::Input { event, occurred_at } = input;
        let feed_url = event.affected_feed_url().clone();
        let previous = tx.load_target_for_endpoint(&feed_url).await?;
        let subscriptions = tx.load_endpoint_subscriptions(&feed_url).await?;
        let target = subscriptions
            .crawl_target_decision()
            .into_target(occurred_at);
        tx.upsert_target(&target).await?;

        debug!(
            feed_url = target.feed_url.as_str(),
            ?target.state,
            "crawl target projected"
        );
        let event = target.lifecycle_event(previous.as_ref());
        if let Some(event) = &event {
            target.log_lifecycle_event(event);
        }
        Ok(event.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::time::Duration;

    use chrono::Utc;
    use synd_feed::types::FeedUrl;

    use crate::{
        crawl::{
            policy::{CrawlPolicy, PollingInterval, PollingPolicy},
            target_list::{
                CrawlTargetState, FeedEndpointSubscription, FeedEndpointSubscriptionSet,
            },
        },
        subscription::{SubscriberId, SubscriptionKey},
    };

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
