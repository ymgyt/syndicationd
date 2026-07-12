use std::sync::Arc;

use futures_util::FutureExt;
use synd_client::payload;
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    application::{FeedApiRef, Populate, RequestId},
    event::{ApiEvent, Event, FeedsApiEvent},
};

use super::runtime::DriverRuntime;

/// Executes feed API requests.
pub(super) struct FeedDriver {
    pub(super) api: FeedApiRef,
}

impl FeedDriver {
    pub(super) fn subscribe_feed(
        &self,
        runtime: &mut DriverRuntime,
        input: payload::SubscribeFeedInput,
    ) {
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::SubscribeFeed);
        let fut = async move {
            match api.subscribe_feed(input).await {
                Ok(_) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedSubscribed),
                }),
                Err(error) => Ok(Event::synd_api_error(error, request_seq)),
            }
        }
        .boxed();
        runtime.push_job(fut);
    }

    /// Returns the `FeedRefreshRequested` event for the accepted request.
    pub(super) fn refresh_feed(&self, runtime: &mut DriverRuntime, url: FeedUrl) -> Event {
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::RefreshFeed);
        let event = Event::FeedRefreshRequested {
            request_seq,
            url: url.clone(),
        };
        let fut = async move {
            match api.refresh_feed(url.clone()).await {
                Ok(payload) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedRefreshAccepted { url, payload }),
                }),
                Err(error) => Ok(Event::synd_api_error(error, request_seq)),
            }
        }
        .boxed();
        runtime.push_job(fut);
        event
    }

    pub(super) fn fetch_feed_refresh_status(
        &self,
        runtime: &mut DriverRuntime,
        url: FeedUrl,
        request_id: String,
        remaining: u16,
    ) {
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::FetchFeedStatus);
        let fut = async move {
            match api.fetch_feed_status(url.clone()).await {
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
        runtime.push_job(fut);
    }

    pub(super) fn unsubscribe_feed(&self, runtime: &mut DriverRuntime, url: FeedUrl) {
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::UnsubscribeFeed);
        let fut = async move {
            match api.unsubscribe_feed(url.clone()).await {
                Ok(()) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Feeds(FeedsApiEvent::FeedUnsubscribed { url }),
                }),
                Err(err) => Ok(Event::synd_api_error(err, request_seq)),
            }
        }
        .boxed();
        runtime.push_job(fut);
    }

    /// Returns the `EntryFetchStarted` event for the started request.
    pub(super) fn fetch_initial_feed_view(
        &self,
        runtime: &mut DriverRuntime,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> Event {
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::FetchSubscription);
        let fut = async move {
            match api
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
        runtime.push_job(fut);
        Event::EntryFetchStarted {
            request_seq,
            populate: Populate::Replace,
        }
    }

    pub(super) fn fetch_subscription(
        &self,
        runtime: &mut DriverRuntime,
        populate: Populate,
        after: Option<String>,
        first: i64,
    ) {
        if first <= 0 {
            return;
        }
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::FetchSubscription);
        let fut = async move {
            match api.fetch_subscription(after, Some(first)).await {
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
        runtime.push_job(fut);
    }

    /// Returns the started events: `EntryFetchStarted` and, for a timeline
    /// refetch, `TimelineRefetchStarted`.
    pub(super) fn fetch_entries(
        &self,
        runtime: &mut DriverRuntime,
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
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::FetchEntries);
        let mut events = vec![Event::EntryFetchStarted {
            request_seq,
            populate,
        }];
        if timeline_refetch {
            events.push(Event::TimelineRefetchStarted { request_seq });
        }
        let fut = async move {
            match api.fetch_entries(after, first).await {
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
        runtime.push_job(fut);
        events
    }
}
