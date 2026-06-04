use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{
    crawl::policy::CrawlPolicy,
    event::{RequestId, SubscribeFeedRequested, UnsubscribeFeedRequested},
    subscription::{SubscriberId, SubscriptionKey},
};

#[derive(Debug, Clone)]
pub struct SubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
}

impl SubscribeFeedCommand {
    pub(crate) fn into_request(self) -> SubscribeFeedRequested {
        SubscribeFeedRequested::new(
            RequestId::generate(),
            SubscriptionKey::new(self.subscriber_id, self.feed_url),
            self.requirement,
            self.category,
            self.crawl_policy,
        )
    }
}

#[derive(Debug, Clone)]
pub struct SubscribeFeedOutput {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl From<SubscribeFeedRequested> for SubscribeFeedOutput {
    fn from(request: SubscribeFeedRequested) -> Self {
        let SubscribeFeedRequested {
            request_id,
            subscription,
            ..
        } = request;

        Self {
            request_id,
            subscription,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}

impl UnsubscribeFeedCommand {
    pub(crate) fn into_request(self) -> UnsubscribeFeedRequested {
        UnsubscribeFeedRequested::new(
            RequestId::generate(),
            SubscriptionKey::new(self.subscriber_id, self.feed_url),
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedOutput {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl From<UnsubscribeFeedRequested> for UnsubscribeFeedOutput {
    fn from(request: UnsubscribeFeedRequested) -> Self {
        let UnsubscribeFeedRequested {
            request_id,
            subscription,
        } = request;

        Self {
            request_id,
            subscription,
        }
    }
}
