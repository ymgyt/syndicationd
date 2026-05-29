use async_graphql::{Context, Result, SimpleObject, Subscription};
use futures_util::{Stream, stream};
use synd_feed::types::FeedUrl;
use synd_registry::event::{
    AffectedFeeds, RegistryNotification, RegistryNotificationRecvError, TimelineChanged,
};

use crate::gql::{registry, subscriber_id};

pub(crate) struct RegistrySubscription;

#[derive(SimpleObject)]
struct TimelineChangedEvent {
    changed_at: crate::gql::scalar::Rfc3339Time,
    affected_feeds: Option<Vec<FeedUrl>>,
}

impl From<TimelineChanged> for TimelineChangedEvent {
    fn from(value: TimelineChanged) -> Self {
        let affected_feeds = match value.affected_feeds {
            AffectedFeeds::Unknown => None,
            AffectedFeeds::Known(feeds) => Some(feeds),
        };

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
    async fn timeline_changed(
        &self,
        cx: &Context<'_>,
    ) -> Result<impl Stream<Item = Result<TimelineChangedEvent>>> {
        let _subscriber_id = subscriber_id(cx);
        let subscriber = registry(cx).subscribe_notifications();

        Ok(stream::unfold(subscriber, |mut subscriber| async move {
            match subscriber.recv().await {
                Ok(RegistryNotification::TimelineChanged(event)) => {
                    Some((Ok(event.into()), subscriber))
                }
                Err(RegistryNotificationRecvError::Lagged(skipped)) => Some((
                    Err(async_graphql::Error::new(format!(
                        "registry event stream lagged by {skipped} messages"
                    ))),
                    subscriber,
                )),
                Err(RegistryNotificationRecvError::Closed) => None,
            }
        }))
    }
}
