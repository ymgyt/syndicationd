use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::policy::PollingPolicy,
    db::{FeedRegistryDb, RegistryTx},
    event::{
        ConsumeContext, Consumer, Event, EventInterests, Processor, ProcessorError, ProcessorId,
        ProcessorResult, SubEvent, SubEventKind, SubscriptionLifecycle, Transactional,
    },
};

/// Current crawl target state derived from active subscriptions for one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlTarget {
    pub feed_url: FeedUrl,
    pub is_active: bool,
    pub polling_policy: Option<PollingPolicy>,
    pub updated_at: DateTime<Utc>,
}

impl CrawlTarget {
    pub fn active(
        feed_url: FeedUrl,
        polling_policy: PollingPolicy,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            is_active: true,
            polling_policy: Some(polling_policy),
            updated_at,
        }
    }

    pub fn inactive(feed_url: FeedUrl, updated_at: DateTime<Utc>) -> Self {
        Self {
            feed_url,
            is_active: false,
            polling_policy: None,
            updated_at,
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
        let subscriptions = cx.list_active_subscriptions_for_feed(&feed_url).await?;
        let target = if let Some(policy) = PollingPolicy::from_subscriptions(&subscriptions) {
            CrawlTarget::active(feed_url.clone(), policy, Utc::now())
        } else {
            CrawlTarget::inactive(feed_url.clone(), Utc::now())
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
