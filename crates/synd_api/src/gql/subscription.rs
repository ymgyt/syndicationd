use async_graphql::{Context, Result, SimpleObject, Subscription, Union};
use futures_util::{Stream, stream};
use synd_feed::types::FeedUrl;
use synd_registry::event::{
    ApiEvent, ApiEventRecvError, ApiFeedSubscribeRejected, ApiFeedSubscribed,
    ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed,
    ApiTimelineChanged,
};

use crate::gql::{registry, scalar, subscriber_id};

pub(crate) struct RegistrySubscription;

#[derive(Union)]
enum FeedEvent {
    Subscribed(FeedSubscribed),
    SubscribeRejected(FeedSubscribeRejected),
    SubscriptionChanged(SubscriptionChanged),
    Unsubscribed(FeedUnsubscribed),
    UnsubscribeRejected(FeedUnsubscribeRejected),
    TimelineChanged(TimelineChanged),
}

#[derive(SimpleObject)]
struct FeedSubscribed {
    request_id: String,
    url: FeedUrl,
}

#[derive(SimpleObject)]
struct FeedSubscribeRejected {
    request_id: String,
    url: FeedUrl,
    reason: String,
}

#[derive(SimpleObject)]
struct SubscriptionChanged {
    request_id: String,
    url: FeedUrl,
}

#[derive(SimpleObject)]
struct FeedUnsubscribed {
    request_id: String,
    url: FeedUrl,
}

#[derive(SimpleObject)]
struct FeedUnsubscribeRejected {
    request_id: String,
    url: FeedUrl,
    reason: String,
}

#[derive(SimpleObject)]
struct TimelineChanged {
    changed_at: scalar::Rfc3339Time,
    affected_feeds: Option<Vec<FeedUrl>>,
}

impl From<ApiFeedSubscribed> for FeedSubscribed {
    fn from(value: ApiFeedSubscribed) -> Self {
        Self {
            request_id: value.request_id.to_string(),
            url: value.subscription.feed_url,
        }
    }
}

impl From<ApiFeedSubscribeRejected> for FeedSubscribeRejected {
    fn from(value: ApiFeedSubscribeRejected) -> Self {
        Self {
            request_id: value.request_id.to_string(),
            url: value.subscription.feed_url,
            reason: value.reason,
        }
    }
}

impl From<ApiFeedSubscriptionChanged> for SubscriptionChanged {
    fn from(value: ApiFeedSubscriptionChanged) -> Self {
        Self {
            request_id: value.request_id.to_string(),
            url: value.subscription.feed_url,
        }
    }
}

impl From<ApiFeedUnsubscribed> for FeedUnsubscribed {
    fn from(value: ApiFeedUnsubscribed) -> Self {
        Self {
            request_id: value.request_id.to_string(),
            url: value.subscription.feed_url,
        }
    }
}

impl From<ApiFeedUnsubscribeRejected> for FeedUnsubscribeRejected {
    fn from(value: ApiFeedUnsubscribeRejected) -> Self {
        Self {
            request_id: value.request_id.to_string(),
            url: value.subscription.feed_url,
            reason: value.reason,
        }
    }
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
        let subscriber = registry(cx).subscribe_api_events(subscriber_id(cx));

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
        ApiEvent::FeedSubscribed(event) => FeedEvent::Subscribed(event.into()),
        ApiEvent::FeedSubscribeRejected(event) => FeedEvent::SubscribeRejected(event.into()),
        ApiEvent::FeedSubscriptionChanged(event) => FeedEvent::SubscriptionChanged(event.into()),
        ApiEvent::FeedUnsubscribed(event) => FeedEvent::Unsubscribed(event.into()),
        ApiEvent::FeedUnsubscribeRejected(event) => FeedEvent::UnsubscribeRejected(event.into()),
        ApiEvent::TimelineChanged(event) => FeedEvent::TimelineChanged(event.into()),
    }
}
