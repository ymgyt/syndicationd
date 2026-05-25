use async_graphql::{Context, Result, SimpleObject, Subscription};
use futures_util::{Stream, stream};
use synd_feed::types::FeedUrl;
use synd_registry::{AffectedFeeds, RegistryEvent, RegistryEventRecvError, TimelineChanged};

use crate::gql::{registry, user_id};

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
    async fn timeline_changed(
        &self,
        cx: &Context<'_>,
    ) -> Result<impl Stream<Item = Result<TimelineChangedEvent>>> {
        let _subscriber_id = user_id(cx)?;
        let subscriber = registry(cx).subscribe_events();

        Ok(stream::unfold(subscriber, |mut subscriber| async move {
            match subscriber.recv().await {
                Ok(RegistryEvent::TimelineChanged(event)) => Some((Ok(event.into()), subscriber)),
                Err(RegistryEventRecvError::Lagged(skipped)) => Some((
                    Err(async_graphql::Error::new(format!(
                        "registry event stream lagged by {skipped} messages"
                    ))),
                    subscriber,
                )),
                Err(RegistryEventRecvError::Closed) => None,
            }
        }))
    }
}
