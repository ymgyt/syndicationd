use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{
    crawl::policy::CrawlPolicy,
    event::RequestId,
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

#[derive(Debug, Clone)]
pub struct SubscribeFeedOutput {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedOutput {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}
