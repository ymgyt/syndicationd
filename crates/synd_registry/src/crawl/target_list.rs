use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::{debug, info};

use crate::{
    crawl::policy::{CrawlPolicy, PollingInterval, PollingPolicy},
    db::{CrawlTargetDb, FeedRegistryDb, SubscriptionDb},
    event::{
        CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent,
        Event, EventInput, EventType, FeedSubscribedEvent, FeedUnsubscribedEvent, Processor,
        ProcessorError, ProcessorId, ProcessorResult, Projector, RegistryEvent, SubEvent,
        SubscriptionChangedEvent,
    },
    subscription::SubscriptionKey,
};

/// Declaration: the crawl instruction for one feed derived from its active
/// subscriptions. Boundary between the subscription world and the crawl world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTarget {
    pub feed_url: FeedUrl,
    pub state: CrawlTargetState,
}

impl CrawlTarget {
    pub fn new(feed_url: FeedUrl, state: CrawlTargetState) -> Self {
        Self { feed_url, state }
    }

    pub fn inactive(feed_url: FeedUrl) -> Self {
        Self::new(feed_url, CrawlTargetState::Inactive)
    }

    /// The lifecycle event represented by the transition from `previous` to
    /// this target, if any.
    pub fn lifecycle_event(&self, previous: Option<&CrawlTarget>) -> Option<Event> {
        match (previous.map(|target| &target.state), &self.state) {
            (
                None | Some(CrawlTargetState::Inactive),
                CrawlTargetState::Active { effective_policy },
            ) => Some(
                CrawlTargetActivatedEvent::new(self.feed_url.clone(), *effective_policy).into(),
            ),
            (
                Some(CrawlTargetState::Active {
                    effective_policy: previous_policy,
                }),
                CrawlTargetState::Active { effective_policy },
            ) if previous_policy != effective_policy => Some(
                CrawlTargetPolicyChangedEvent::new(self.feed_url.clone(), *effective_policy).into(),
            ),
            (Some(CrawlTargetState::Active { .. }), CrawlTargetState::Inactive) => {
                Some(CrawlTargetDeactivatedEvent::new(self.feed_url.clone()).into())
            }
            _ => None,
        }
    }

    fn log_lifecycle_event(event: &Event) {
        match event {
            Event::CrawlTargetActivated(event) => {
                info!(
                    feed_url = event.feed_url.as_str(),
                    policy = %event.policy.polling,
                    "crawl target activated"
                );
            }
            Event::CrawlTargetPolicyChanged(event) => {
                info!(
                    feed_url = event.feed_url.as_str(),
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

/// Whether a feed should be crawled, and under which effective policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlTargetState {
    Active { effective_policy: CrawlPolicy },
    Inactive,
}

/// One subscription's crawl policy over a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionPolicy {
    pub subscription: SubscriptionKey,
    pub crawl_policy: CrawlPolicy,
}

impl SubscriptionPolicy {
    pub fn new(subscription: SubscriptionKey, crawl_policy: CrawlPolicy) -> Self {
        Self {
            subscription,
            crawl_policy,
        }
    }
}

/// Current subscription relations over one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedSubscriptions {
    pub feed_url: FeedUrl,
    pub subscriptions: Vec<SubscriptionPolicy>,
}

impl FeedSubscriptions {
    pub fn new(feed_url: FeedUrl, subscriptions: Vec<SubscriptionPolicy>) -> Self {
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

    pub fn crawl_target_decision(self) -> CrawlTarget {
        let state = if self.subscriptions.is_empty() {
            CrawlTargetState::Inactive
        } else {
            CrawlTargetState::Active {
                effective_policy: self.effective_policy(),
            }
        };

        CrawlTarget::new(self.feed_url, state)
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
    for<'tx> S::Tx<'tx>: CrawlTargetDb + SubscriptionDb + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let Self::Input { event, .. } = input;
        let feed_url = event.affected_feed_url().clone();
        let previous = tx.load_target(&feed_url).await?;
        let subscriptions = tx.load_feed_subscriptions(&feed_url).await?;
        let target = subscriptions.crawl_target_decision();
        tx.upsert_target(&target).await?;

        debug!(
            feed_url = target.feed_url.as_str(),
            ?target.state,
            "crawl target projected"
        );
        let event = target.lifecycle_event(previous.as_ref());
        if let Some(event) = &event {
            CrawlTarget::log_lifecycle_event(event);
        }
        Ok(event.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_feed::types::FeedUrl;

    use crate::{
        crawl::{
            policy::{CrawlPolicy, PollingInterval},
            target_list::{CrawlTargetState, FeedSubscriptions, SubscriptionPolicy},
        },
        subscription::{SubscriberId, SubscriptionKey},
    };

    fn interval(duration: Duration) -> PollingInterval {
        PollingInterval::try_from(duration).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn subscription(subscriber_id: &'static str, crawl_policy: CrawlPolicy) -> SubscriptionPolicy {
        SubscriptionPolicy::new(
            SubscriptionKey::new(SubscriberId::new(subscriber_id), feed_url()),
            crawl_policy,
        )
    }

    fn subscription_set(subscriptions: Vec<SubscriptionPolicy>) -> FeedSubscriptions {
        FeedSubscriptions::new(feed_url(), subscriptions)
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

        let target = subscriptions.crawl_target_decision();

        assert_eq!(
            target.state,
            CrawlTargetState::Active {
                effective_policy: CrawlPolicy::interval(interval(Duration::from_mins(10))),
            }
        );
    }

    #[test]
    fn crawl_target_decision_is_manual_when_all_subscriptions_are_manual() {
        let subscriptions = subscription_set(vec![subscription("manual", CrawlPolicy::manual())]);

        let target = subscriptions.crawl_target_decision();

        assert_eq!(
            target.state,
            CrawlTargetState::Active {
                effective_policy: CrawlPolicy::manual(),
            }
        );
    }

    #[test]
    fn crawl_target_decision_is_inactive_without_subscriptions() {
        let subscriptions = subscription_set(Vec::new());

        let target = subscriptions.crawl_target_decision();

        assert_eq!(target.state, CrawlTargetState::Inactive);
    }
}
