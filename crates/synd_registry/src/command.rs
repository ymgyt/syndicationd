use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{
    crawl::policy::RefreshPolicy, event::RequestId, subscriber::SubscriberId,
    subscription::Subscription,
};

#[derive(Debug, Clone)]
pub struct SubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub refresh_policy: RefreshPolicy,
}

#[derive(Debug, Clone)]
pub struct SubscribeFeedOutput {
    pub subscription: Subscription,
    pub request_id: RequestId,
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedOutput {
    pub request_id: Option<RequestId>,
}
