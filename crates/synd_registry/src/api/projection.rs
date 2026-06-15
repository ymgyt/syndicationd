use chrono::{DateTime, Utc};

use crate::{
    api::{
        ApiCrawlJobEnqueued, ApiCrawlJobFinished, ApiCrawlJobStarted, ApiEntryChanged,
        ApiEntryDiscovered, ApiEvent, ApiFeedChanged, ApiFeedDiscovered, ApiFeedSubscribeRejected,
        ApiFeedSubscribed, ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected,
        ApiFeedUnsubscribed, ApiTimelineChanged,
    },
    db::{CrawlCompletionTx, FeedRegistryDb, SubscriptionTx},
    event::{
        ConsumeContext, Consumer, ConsumerInput, CrawlJobEnqueuedEvent, CrawlJobFinishedEvent,
        CrawlJobStartedEvent, EntryChangedEvent, EntryDiscoveredEvent, Event, EventType,
        FeedChangedEvent, FeedDiscoveredEvent, FeedSubscribedEvent, FeedUnsubscribedEvent,
        Processor, ProcessorError, ProcessorId, ProcessorResult, RegistryEvent,
        SubscribeFeedRejected, SubscriptionChangedEvent, TimelineChangedEvent,
        UnsubscribeFeedRejected,
    },
};

/// Event input used to project public API events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiEventProjectionInput {
    SubscribeFeedRejected(SubscribeFeedRejected),
    UnsubscribeFeedRejected(UnsubscribeFeedRejected),
    FeedSubscribed(FeedSubscribedEvent),
    SubscriptionChanged(SubscriptionChangedEvent),
    FeedUnsubscribed(FeedUnsubscribedEvent),
    CrawlJobEnqueued(CrawlJobEnqueuedEvent),
    CrawlJobStarted(CrawlJobStartedEvent),
    CrawlJobFinished(CrawlJobFinishedEvent),
    FeedDiscovered(FeedDiscoveredEvent),
    FeedChanged(FeedChangedEvent),
    EntryDiscovered(EntryDiscoveredEvent),
    EntryChanged(EntryChangedEvent),
    TimelineChanged(TimelineChangedEvent),
}

impl ApiEventProjectionInput {
    async fn into_api_events<Tx>(
        self,
        cx: &mut ConsumeContext<'_, Tx>,
    ) -> ProcessorResult<Vec<Event>>
    where
        Tx: CrawlCompletionTx + SubscriptionTx + Send,
    {
        match self {
            Self::SubscribeFeedRejected(event) => Ok(vec![
                ApiEvent::FeedSubscribeRejected(ApiFeedSubscribeRejected::new(
                    event.request_id,
                    event.subscription,
                    event.reason,
                ))
                .into(),
            ]),
            Self::UnsubscribeFeedRejected(event) => Ok(vec![
                ApiEvent::FeedUnsubscribeRejected(ApiFeedUnsubscribeRejected::new(
                    event.request_id,
                    event.subscription,
                    event.reason,
                ))
                .into(),
            ]),
            Self::FeedSubscribed(event) => {
                let Some(request_id) = event.request_id else {
                    return Ok(Vec::new());
                };
                Ok(vec![
                    ApiEvent::FeedSubscribed(ApiFeedSubscribed::new(
                        request_id,
                        event.subscription,
                    ))
                    .into(),
                ])
            }
            Self::SubscriptionChanged(event) => {
                let Some(request_id) = event.request_id else {
                    return Ok(Vec::new());
                };
                Ok(vec![
                    ApiEvent::FeedSubscriptionChanged(ApiFeedSubscriptionChanged::new(
                        request_id,
                        event.subscription,
                    ))
                    .into(),
                ])
            }
            Self::FeedUnsubscribed(event) => {
                let Some(request_id) = event.request_id else {
                    return Ok(Vec::new());
                };
                Ok(vec![
                    ApiEvent::FeedUnsubscribed(ApiFeedUnsubscribed::new(
                        request_id,
                        event.subscription,
                    ))
                    .into(),
                ])
            }
            Self::CrawlJobEnqueued(event) => {
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::CrawlJobEnqueued(ApiCrawlJobEnqueued::new(
                        subscriber_id,
                        event.feed_url.clone(),
                    ))
                })
                .await
            }
            Self::CrawlJobStarted(event) => {
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::CrawlJobStarted(ApiCrawlJobStarted::new(
                        subscriber_id,
                        event.feed_url.clone(),
                    ))
                })
                .await
            }
            Self::CrawlJobFinished(event) => {
                let state = cx.load_crawl_state(&event.feed_url).await?;
                let (http_status, error) = state.map_or((None, None), |state| {
                    (
                        state
                            .last
                            .http_status
                            .map(synd_feed::feed::service::FeedHttpStatus::as_u16),
                        state.last.error.map(|error| error.kind.to_string()),
                    )
                });
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::CrawlJobFinished(ApiCrawlJobFinished::new(
                        subscriber_id,
                        event.feed_url.clone(),
                        http_status,
                        error.clone(),
                    ))
                })
                .await
            }
            Self::FeedDiscovered(event) => {
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::FeedDiscovered(ApiFeedDiscovered::new(
                        subscriber_id,
                        event.feed_url.clone(),
                    ))
                })
                .await
            }
            Self::FeedChanged(event) => {
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::FeedChanged(ApiFeedChanged::new(
                        subscriber_id,
                        event.feed_url.clone(),
                    ))
                })
                .await
            }
            Self::EntryDiscovered(event) => {
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::EntryDiscovered(ApiEntryDiscovered::new(
                        subscriber_id,
                        event.feed_url.clone(),
                    ))
                })
                .await
            }
            Self::EntryChanged(event) => {
                feed_subscriber_events(cx, &event.feed_url, |subscriber_id| {
                    ApiEvent::EntryChanged(ApiEntryChanged::new(
                        subscriber_id,
                        event.feed_url.clone(),
                    ))
                })
                .await
            }
            Self::TimelineChanged(event) => Ok(vec![
                ApiEvent::TimelineChanged(ApiTimelineChanged::new(
                    event.timeline,
                    event.changed_at,
                    event.affected_feeds,
                ))
                .into(),
            ]),
        }
    }
}

impl ConsumerInput for ApiEventProjectionInput {
    const INTERESTS: &'static [EventType] = &[
        SubscribeFeedRejected::TYPE,
        UnsubscribeFeedRejected::TYPE,
        FeedSubscribedEvent::TYPE,
        SubscriptionChangedEvent::TYPE,
        FeedUnsubscribedEvent::TYPE,
        CrawlJobEnqueuedEvent::TYPE,
        CrawlJobStartedEvent::TYPE,
        CrawlJobFinishedEvent::TYPE,
        FeedDiscoveredEvent::TYPE,
        FeedChangedEvent::TYPE,
        EntryDiscoveredEvent::TYPE,
        EntryChangedEvent::TYPE,
        TimelineChangedEvent::TYPE,
    ];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::SubscribeFeedRejected(event) => Ok(Self::SubscribeFeedRejected(event)),
            Event::UnsubscribeFeedRejected(event) => Ok(Self::UnsubscribeFeedRejected(event)),
            Event::FeedSubscribed(event) => Ok(Self::FeedSubscribed(event)),
            Event::SubscriptionChanged(event) => Ok(Self::SubscriptionChanged(event)),
            Event::FeedUnsubscribed(event) => Ok(Self::FeedUnsubscribed(event)),
            Event::CrawlJobEnqueued(event) => Ok(Self::CrawlJobEnqueued(event)),
            Event::CrawlJobStarted(event) => Ok(Self::CrawlJobStarted(event)),
            Event::CrawlJobFinished(event) => Ok(Self::CrawlJobFinished(event)),
            Event::FeedDiscovered(event) => Ok(Self::FeedDiscovered(event)),
            Event::FeedChanged(event) => Ok(Self::FeedChanged(event)),
            Event::EntryDiscovered(event) => Ok(Self::EntryDiscovered(event)),
            Event::EntryChanged(event) => Ok(Self::EntryChanged(event)),
            Event::TimelineChanged(event) => Ok(Self::TimelineChanged(event)),
            event => Err(ProcessorError::unexpected_input(
                "api projection event",
                &event,
            )),
        }
    }
}

/// Projects request and subscription facts into public API events.
#[derive(Debug, Clone)]
pub struct ApiEventProj;

impl ApiEventProj {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiEventProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for ApiEventProj {
    type Input = ApiEventProjectionInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::ApiEventProjection
    }
}

impl<S> Consumer<S> for ApiEventProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlCompletionTx + SubscriptionTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        input.into_api_events(cx).await
    }
}

async fn feed_subscriber_events<Tx>(
    cx: &mut ConsumeContext<'_, Tx>,
    feed_url: &synd_feed::types::FeedUrl,
    build: impl Fn(crate::SubscriberId) -> ApiEvent,
) -> ProcessorResult<Vec<Event>>
where
    Tx: SubscriptionTx + Send,
{
    let subscriptions = cx.load_feed_endpoint_subscriptions(feed_url).await?;
    Ok(subscriptions
        .subscriptions
        .into_iter()
        .map(|subscription| build(subscription.subscription.subscriber_id).into())
        .collect())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::{
        CommitTx, FeedRegistryDb, FeedSubscriptionAttrs, InMemoryFeedRegistryDb, SubscriberId,
        SubscriptionKey, SubscriptionTx,
        crawl::{
            job::CrawlJobId,
            policy::{CrawlPolicy, PollingInterval},
        },
    };

    #[tokio::test]
    async fn feed_scoped_events_are_projected_for_active_subscribers() -> anyhow::Result<()> {
        let db = InMemoryFeedRegistryDb::new();
        let subscriber_id = SubscriberId::new("local");
        let feed_url = FeedUrl::parse("https://example.com/feed.xml")?;
        let now = Utc.with_ymd_and_hms(2026, 6, 15, 12, 0, 0).unwrap();

        let mut tx = db.begin().await?;
        tx.upsert_feed_endpoint(&feed_url, now).await?;
        tx.upsert_feed_subscription(
            &SubscriptionKey::new(subscriber_id.clone(), feed_url.clone()),
            FeedSubscriptionAttrs {
                requirement: None,
                category: None,
                crawl_policy: CrawlPolicy::interval(PollingInterval::try_from(
                    Duration::from_hours(1),
                )?),
            },
            now,
        )
        .await?;

        let mut projection = ApiEventProj::new();
        let events = {
            let mut cx = ConsumeContext::new(&mut tx);
            <ApiEventProj as Consumer<InMemoryFeedRegistryDb>>::consume(
                &mut projection,
                &mut cx,
                ApiEventProjectionInput::CrawlJobStarted(CrawlJobStartedEvent::new(
                    CrawlJobId::new("job"),
                    feed_url.clone(),
                )),
            )
            .await?
        };
        tx.commit().await?;

        assert_eq!(events.len(), 1);
        let Event::ApiCrawlJobStarted(event) = &events[0] else {
            anyhow::bail!("unexpected event: {:?}", events[0]);
        };
        assert_eq!(event.subscriber_id, subscriber_id);
        assert_eq!(event.feed_url, feed_url);
        Ok(())
    }
}
