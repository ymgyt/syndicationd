use async_graphql::{Context, Result, SimpleObject, Subscription, Union};
use futures_util::{Stream, stream};
use synd_feed::types::FeedUrl;
use synd_registry::api::{ApiEvent, ApiEventRecvError, ApiTimelineChanged};

use crate::gql::{registry, scalar, subscriber_id};

pub(crate) struct RegistrySubscription;

#[derive(Union)]
enum FeedEvent {
    TimelineChanged(TimelineChanged),
}

#[derive(SimpleObject)]
struct TimelineChanged {
    changed_at: scalar::Rfc3339Time,
    affected_feeds: Option<Vec<FeedUrl>>,
}

impl From<ApiTimelineChanged> for TimelineChanged {
    fn from(value: ApiTimelineChanged) -> Self {
        let affected_feeds = (!value.affected_feeds.is_empty()).then_some(value.affected_feeds);
        Self {
            changed_at: value.changed_at.into(),
            affected_feeds,
        }
    }
}

#[Subscription]
impl RegistrySubscription {
    // async-graphql requires subscription stream resolvers to be async.
    #[allow(clippy::unused_async)]
    async fn feed_events(&self, cx: &Context<'_>) -> Result<impl Stream<Item = Result<FeedEvent>>> {
        let subscriber = registry(cx).subscribe_events(subscriber_id(cx));

        Ok(stream::unfold(subscriber, |mut subscriber| async move {
            match subscriber.recv().await {
                Ok(event) => Some((Ok(feed_event_from_api_event(event)), subscriber)),
                Err(ApiEventRecvError::Lagged(skipped)) => Some((
                    Err(async_graphql::Error::new(format!(
                        "feed event stream lagged by {skipped} messages"
                    ))),
                    subscriber,
                )),
                Err(ApiEventRecvError::Closed) => None,
            }
        }))
    }
}

fn feed_event_from_api_event(event: ApiEvent) -> FeedEvent {
    match event {
        ApiEvent::TimelineChanged(event) => FeedEvent::TimelineChanged(event.into()),
    }
}
