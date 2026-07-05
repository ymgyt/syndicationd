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
    FeedEvents(Result<Vec<payload::FeedEvent>, SyndApiError>),
}

/// In-memory `FeedApi` implementation for terminal workflow tests.
pub struct MockFeedApi {
    responses: Mutex<VecDeque<MockFeedApiResponse>>,
}

impl MockFeedApi {
    pub fn new(responses: impl IntoIterator<Item = MockFeedApiResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
        }
    }

    fn pop_response(
        &self,
        mut matches: impl FnMut(&MockFeedApiResponse) -> bool,
    ) -> Result<MockFeedApiResponse, SyndApiError> {
        let mut responses = self
            .responses
            .lock()
            .expect("mock feed API response lock must not be poisoned");

        if let Some(index) = responses.iter().position(&mut matches) {
            return responses
                .remove(index)
                .ok_or(SyndApiError::UnexpectedResponse {
                    context: "mock feed API response queue index is invalid",
                });
        }

        if responses.is_empty() {
            Err(SyndApiError::UnexpectedResponse {
                context: "mock feed API response queue is empty",
            })
        } else {
            Err(Self::mismatch())
        }
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

    fn fetch_initial_feed_view(
        &self,
        _subscriptions_first: i64,
        _timeline_first: i64,
    ) -> BoxFuture<'static, Result<payload::InitialFeedViewPayload, SyndApiError>> {
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::InitialFeedView(_)))
        {
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
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::Subscription(_)))
        {
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
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::SubscribeFeed(_)))
        {
            Ok(MockFeedApiResponse::SubscribeFeed(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn unsubscribe_feed(&self, _url: FeedUrl) -> BoxFuture<'static, Result<(), SyndApiError>> {
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::UnsubscribeFeed(_)))
        {
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
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::RefreshFeed(_)))
        {
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
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::FeedStatus(_)))
        {
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
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::Entries(_)))
        {
            Ok(MockFeedApiResponse::Entries(result)) => result,
            Ok(_) => Err(Self::mismatch()),
            Err(err) => Err(err),
        };
        future::ready(result).boxed()
    }

    fn run_feed_events(
        &self,
        events: mpsc::UnboundedSender<payload::FeedEvent>,
    ) -> BoxFuture<'static, Result<(), SyndApiError>> {
        let result = match self
            .pop_response(|response| matches!(response, MockFeedApiResponse::FeedEvents(_)))
        {
            Ok(MockFeedApiResponse::FeedEvents(Ok(feed_events))) => {
                for event in feed_events {
                    if events.send(event).is_err() {
                        break;
                    }
                }
                Ok(())
            }
            Ok(MockFeedApiResponse::FeedEvents(Err(err))) | Err(err) => Err(err),
            Ok(_) => Err(Self::mismatch()),
        };
        future::ready(result).boxed()
    }
}
