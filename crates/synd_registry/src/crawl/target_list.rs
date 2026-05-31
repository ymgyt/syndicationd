use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::policy::PollingPolicy,
    db::{FeedRegistryDb, RegistryDbTransaction},
    event::{
        ConsumerEventInput, Event, EventConsumer, EventConsumerId, EventConsumerResult, EventKind,
        EventReadBatch, EventReadFilter, JournaledEvent, RecordedEvents, SubEvent, SubEventKind,
        SubscriptionLifecycle,
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
    events: Vec<SubscriptionLifecycle>,
}

impl CrawlTargetListInput {
    pub fn new(events: Vec<SubscriptionLifecycle>) -> Self {
        Self { events }
    }

    pub fn into_events(self) -> Vec<SubscriptionLifecycle> {
        self.events
    }
}

impl ConsumerEventInput for CrawlTargetListInput {
    const READ_FILTER: EventReadFilter = EventReadFilter::new(&[
        EventKind::Sub(SubEventKind::FeedSubscribed),
        EventKind::Sub(SubEventKind::SubscriptionChanged),
        EventKind::Sub(SubEventKind::FeedUnsubscribed),
    ]);

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>> {
        let events = batch
            .into_events()
            .into_iter()
            .map(JournaledEvent::into_event)
            .map(|event| match event {
                Event::Sub(SubEvent::FeedSubscribed(event)) => {
                    SubscriptionLifecycle::Subscribed(event)
                }
                Event::Sub(SubEvent::SubscriptionChanged(event)) => {
                    SubscriptionLifecycle::Changed(event)
                }
                Event::Sub(SubEvent::FeedUnsubscribed(event)) => {
                    SubscriptionLifecycle::Unsubscribed(event)
                }
                event => unreachable!("unexpected crawl target list event: {event:?}"),
            })
            .collect::<Vec<_>>();

        Ok((!events.is_empty()).then_some(Self::new(events)))
    }
}

/// Consumer that reacts to subscription events for the crawl target list.
#[derive(Debug, Clone)]
pub struct CrawlTargetListProj<S> {
    db: S,
}

impl<S> CrawlTargetListProj<S> {
    pub fn new(db: S) -> Self {
        Self { db }
    }
}

impl<S> EventConsumer for CrawlTargetListProj<S>
where
    S: FeedRegistryDb,
{
    type Input = CrawlTargetListInput;

    fn id(&self) -> EventConsumerId {
        EventConsumerId::CrawlTargetListProj
    }

    async fn consume(&mut self, input: Self::Input) -> EventConsumerResult<RecordedEvents> {
        let feed_urls = affected_feed_urls(input.into_events());
        if feed_urls.is_empty() {
            return Ok(RecordedEvents::empty());
        }

        let mut tx = self.db.begin().await?;
        for feed_url in &feed_urls {
            let subscriptions = tx.list_active_subscriptions_for_feed(feed_url).await?;
            let target = if let Some(policy) = PollingPolicy::from_subscriptions(&subscriptions) {
                CrawlTarget::active(feed_url.clone(), policy, Utc::now())
            } else {
                CrawlTarget::inactive(feed_url.clone(), Utc::now())
            };
            tx.upsert_crawl_target(target).await?;
        }
        tx.commit().await?;

        debug!(
            feed_count = feed_urls.len(),
            "crawl target list projector reconciled feeds"
        );
        Ok(RecordedEvents::empty())
    }
}

fn affected_feed_urls(events: Vec<SubscriptionLifecycle>) -> Vec<FeedUrl> {
    let mut feed_urls = Vec::new();
    for event in events {
        let feed_url = match event {
            SubscriptionLifecycle::Subscribed(event) => event.subscription.feed_url,
            SubscriptionLifecycle::Changed(event) => event.subscription.feed_url,
            SubscriptionLifecycle::Unsubscribed(event) => event.subscription.feed_url,
        };
        if !feed_urls.contains(&feed_url) {
            feed_urls.push(feed_url);
        }
    }

    feed_urls.sort_unstable_by(|a, b| a.as_str().cmp(b.as_str()));
    feed_urls
}
