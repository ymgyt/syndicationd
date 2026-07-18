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

    /// Returns the `EntryFetchStarted` event for the started request.
    pub(super) fn fetch_entries(
        &self,
        runtime: &mut DriverRuntime,
        populate: Populate,
        after: Option<String>,
        first: i64,
    ) -> Option<Event> {
        if first <= 0 {
            return None;
        }
        debug!(?populate, has_after = after.is_some(), first, "fetch entries");
        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::FetchEntries);
        let fut = async move {
            match api.fetch_timeline_entries(after, first).await {
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
        Some(Event::EntryFetchStarted {
            request_seq,
            populate,
        })
    }

    /// Fetch and coalesce all timeline changes after `since` in one job.
    pub(super) fn sync_timeline(&self, runtime: &mut DriverRuntime, since: i64) {
        const CHANGES_PER_PAGE: i64 = 200;

        let api = self.api.clone();
        let request_seq = runtime.request_started(RequestId::SyncTimeline);
        let fut = async move {
            let mut changes = Vec::new();
            let mut since = since;
            loop {
                match api.fetch_timeline_changes(since, CHANGES_PER_PAGE).await {
                    Ok(mut page) => {
                        changes.append(&mut page.changes);
                        since = page.seq;
                        if !page.has_more {
                            return Ok(Event::Api {
                                request_seq,
                                event: ApiEvent::Feeds(FeedsApiEvent::TimelineChangesFetched {
                                    changes,
                                    seq: page.seq,
                                }),
                            });
                        }
                    }
                    Err(error) => return Ok(Event::synd_api_error(error, request_seq)),
                }
            }
        }
        .boxed();
        runtime.push_job(fut);
    }
}
