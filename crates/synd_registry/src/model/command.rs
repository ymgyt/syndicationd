use synd_feed::types::{Category, FeedUrl, Requirement};

use super::{FeedSubscription, RefreshPolicy, RefreshRequestReceipt, RefreshStatus, SubscriberId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialRefreshMode {
    Async,
    RequireSuccess,
}

#[derive(Debug, Clone)]
pub struct SubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub refresh_policy: RefreshPolicy,
    pub initial_refresh: InitialRefreshMode,
}

#[derive(Debug, Clone)]
pub struct SubscribeFeedOutput {
    pub subscription: FeedSubscription,
    pub refresh: SubscribeFeedRefresh,
}

#[derive(Debug, Clone)]
pub enum SubscribeFeedRefresh {
    Enqueued(RefreshRequestReceipt),
    Completed(RefreshStatus),
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedOutput {}

#[derive(Debug, Clone)]
pub struct RequestRefreshCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}
