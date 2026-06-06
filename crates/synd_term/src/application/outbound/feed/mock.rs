use std::{collections::VecDeque, future, sync::Mutex};

use futures_util::{FutureExt as _, future::BoxFuture};
use synd_client::{ApiCredential, SyndApiError, payload};
use synd_feed::types::FeedUrl;
use tokio::sync::mpsc;

use super::FeedApi;

pub enum MockFeedApiResponse {
    InitialFeedView(Result<payload::InitialFeedViewPayload, SyndApiError>),
    Subscription(Result<payload::SubscriptionPayload, SyndApiError>),
    SubscribeFeed(Result<payload::SubscribeFeedPayload, SyndApiError>),
    UnsubscribeFeed(Result<(), SyndApiError>),
    RefreshFeed(Result<payload::RefreshFeedPayload, SyndApiError>),
    FeedStatus(Result<payload::RefreshStatus, SyndApiError>),
    Entries(Result<payload::FetchEntriesPayload, SyndApiError>),
    TimelineChanges(Result<Vec<payload::TimelineChangeEvent>, SyndApiError>),
}

/// In-memory `FeedApi` implementation for terminal workflow tests.
pub struct MockFeedApi {
    responses: Mutex<VecDeque<MockFeedApiResponse>>,
    supports_timeline_change_subscription: bool,
}

impl MockFeedApi {
    pub fn new(responses: impl IntoIterator<Item = MockFeedApiResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            supports_timeline_change_subscription: false,
        }
    }

    #[must_use]
    pub fn with_timeline_change_subscription(mut self) -> Self {
        self.supports_timeline_change_subscription = true;
        self
    }

    fn pop_response(&self) -> Result<MockFeedApiResponse, SyndApiError> {
        self.responses
            .lock()
            .expect("mock feed API response lock must not be poisoned")
            .pop_front()
            .ok_or(SyndApiError::UnexpectedResponse {
                context: "mock feed API response queue is empty",
            })
    }

    fn mismatch() -> SyndApiError {
        SyndApiError::UnexpectedResponse {
            context: "mock feed API response does not match request",
        }
    }
}

impl FeedApi for MockFeedApi {
    fn set_credential(&self, _credential: ApiCredential) -> Result<(), SyndApiError> {
        Ok(())
    }

    fn supports_timeline_change_subscription(&self) -> bool {
        self.supports_timeline_change_subscription
    }

    fn fetch_initial_feed_view(
        &self,
        _subscriptions_first: i64,
        _timeline_first: i64,
    ) -> BoxFuture<'static, Result<payload::InitialFeedViewPayload, SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::InitialFeedView(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn fetch_subscription(
        &self,
        _after: Option<String>,
        _first: Option<i64>,
    ) -> BoxFuture<'static, Result<payload::SubscriptionPayload, SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::Subscription(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn subscribe_feed(
        &self,
        _input: payload::SubscribeFeedInput,
    ) -> BoxFuture<'static, Result<payload::SubscribeFeedPayload, SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::SubscribeFeed(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn unsubscribe_feed(&self, _url: FeedUrl) -> BoxFuture<'static, Result<(), SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::UnsubscribeFeed(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn refresh_feed(
        &self,
        _url: FeedUrl,
    ) -> BoxFuture<'static, Result<payload::RefreshFeedPayload, SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::RefreshFeed(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn fetch_feed_status(
        &self,
        _url: FeedUrl,
    ) -> BoxFuture<'static, Result<payload::RefreshStatus, SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::FeedStatus(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn fetch_entries(
        &self,
        _after: Option<String>,
        _first: i64,
    ) -> BoxFuture<'static, Result<payload::FetchEntriesPayload, SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::Entries(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn run_timeline_changes(
        &self,
        events: mpsc::UnboundedSender<payload::TimelineChangeEvent>,
    ) -> BoxFuture<'static, Result<(), SyndApiError>> {
        let result = match self.pop_response() {
            Ok(MockFeedApiResponse::TimelineChanges(Ok(changes))) => {
                for event in changes {
                    if events.send(event).is_err() {
                        break;
                    }
                }
                Ok(())
            }
            Ok(MockFeedApiResponse::TimelineChanges(Err(err))) | Err(err) => Err(err),
            Ok(_) => Err(Self::mismatch()),
        };
        future::ready(result).boxed()
    }
}
