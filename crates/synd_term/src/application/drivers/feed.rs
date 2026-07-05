use std::sync::Arc;

use futures_util::FutureExt;
use synd_client::payload;
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    application::{
        FEED_REFRESH_POLL_INTERVAL, FEED_VIEW_SYNC_INTERVAL, Populate, RequestId,
        TIMELINE_INVALIDATION_DEBOUNCE,
    },
    event::{ApiEvent, Event, FeedsApiEvent},
};

use super::DriverContext;

pub(super) struct FeedDriver;

impl FeedDriver {
    pub(super) fn subscribe_feed(
        cx: &mut DriverContext<'_>,
        input: payload::SubscribeFeedInput,
    ) -> Vec<Event> {
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::SubscribeFeed);
        let fut = async move {
            match feed_api.subscribe_feed(input).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedSubscribed {
                        url: payload.url.clone(),
                        payload,
                    }),
                }),
                Err(error) => Ok(Event::synd_api_error(error, request_seq)),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn refresh_feed(cx: &mut DriverContext<'_>, url: FeedUrl) -> Vec<Event> {
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::RefreshFeed);
        let event = Event::FeedRefreshRequested {
            request_seq,
            url: url.clone(),
        };
        let fut = async move {
            match feed_api.refresh_feed(url.clone()).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedRefreshAccepted { url, payload }),
                }),
                Err(error) => Ok(Event::synd_api_error(error, request_seq)),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        vec![event]
    }

    pub(super) fn schedule_feed_refresh_poll(
        cx: &mut DriverContext<'_>,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
    ) -> Vec<Event> {
        let fut = async move {
            tokio::time::sleep(FEED_REFRESH_POLL_INTERVAL).await;
            Ok(Event::FeedRefreshPollElapsed {
                url,
                request_id,
                remaining,
            })
        }
        .boxed();
        cx.runtime.push_background_job(fut);
        Vec::new()
    }

    pub(super) fn fetch_feed_refresh_status(
        cx: &mut DriverContext<'_>,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
    ) -> Vec<Event> {
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::FetchFeedStatus);
        let fut = async move {
            match feed_api.fetch_feed_status(url.clone()).await {
                Ok(status) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedRefreshStatusFetched {
                        url,
                        request_id: request_id.clone(),
                        remaining,
                        status,
                    }),
                }),
                Err(err) => Ok(Event::FeedRefreshPollError {
                    url,
                    request_id,
                    error: Arc::new(err),
                    request_seq,
                }),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn unsubscribe_feed(cx: &mut DriverContext<'_>, url: FeedUrl) -> Vec<Event> {
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::UnsubscribeFeed);
        let fut = async move {
            match feed_api.unsubscribe_feed(url.clone()).await {
                Ok(()) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedUnsubscribed { url }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn fetch_initial_feed_view(
        cx: &mut DriverContext<'_>,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> Vec<Event> {
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::FetchSubscription);
        let fut = async move {
            match feed_api
                .fetch_initial_feed_view(subscriptions_first, timeline_first)
                .await
            {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::InitialFeedViewFetched { payload }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        vec![Event::EntryFetchStarted {
            request_seq,
            populate: Populate::Replace,
        }]
    }

    pub(super) fn fetch_subscription(
        cx: &mut DriverContext<'_>,
        populate: Populate,
        after: Option<String>,
        first: i64,
    ) -> Vec<Event> {
        if first <= 0 {
            return Vec::new();
        }
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::FetchSubscription);
        let fut = async move {
            match feed_api.fetch_subscription(after, Some(first)).await {
                Ok(subscription) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::SubscriptionFetched {
                        populate,
                        subscription,
                    }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn fetch_entries(
        cx: &mut DriverContext<'_>,
        populate: Populate,
        after: Option<String>,
        first: i64,
        timeline_refetch: bool,
    ) -> Vec<Event> {
        if first <= 0 {
            return Vec::new();
        }
        debug!(
            ?populate,
            has_after = after.is_some(),
            first,
            timeline_refetch,
            "fetch entries"
        );
        let feed_api = cx.handles.feed_api.clone();
        let request_seq = cx.runtime.request_started(RequestId::FetchEntries);
        let mut events = vec![Event::EntryFetchStarted {
            request_seq,
            populate,
        }];
        if timeline_refetch {
            events.push(Event::TimelineRefetchStarted { request_seq });
        }
        let fut = async move {
            match feed_api.fetch_entries(after, first).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::EntriesFetched { populate, payload }),
                }),
                Err(error) => Ok(Event::SyndApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        events
    }

    pub(super) fn schedule_feed_view_sync(cx: &mut DriverContext<'_>) -> Vec<Event> {
        let fut = async move {
            tokio::time::sleep(FEED_VIEW_SYNC_INTERVAL).await;
            Ok(Event::FeedViewSyncElapsed)
        }
        .boxed();
        cx.runtime.push_background_job(fut);
        Vec::new()
    }

    pub(super) fn schedule_feed_view_reload(
        cx: &mut DriverContext<'_>,
        feeds_first: i64,
        entries_first: i64,
    ) -> Vec<Event> {
        let fut = async move {
            tokio::time::sleep(TIMELINE_INVALIDATION_DEBOUNCE).await;
            Ok(Event::FeedViewReloadDebounced {
                feeds_first,
                entries_first,
            })
        }
        .boxed();
        cx.runtime.push_background_job(fut);
        Vec::new()
    }

    pub(super) fn schedule_timeline_reload(cx: &mut DriverContext<'_>) -> Vec<Event> {
        let fut = async move {
            tokio::time::sleep(TIMELINE_INVALIDATION_DEBOUNCE).await;
            Ok(Event::TimelineReloadDebounced)
        }
        .boxed();
        cx.runtime.push_background_job(fut);
        Vec::new()
    }
}
