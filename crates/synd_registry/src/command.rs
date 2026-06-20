use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{
    crawl::policy::CrawlPolicy,
    subscription::{
        FeedSubscriptionAttrs, SubscribeOutcome, SubscriberId, SubscriptionKey, UnsubscribeOutcome,
    },
};

#[derive(Debug, Clone)]
pub struct SubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: Option<CrawlPolicy>,
}

impl SubscribeFeedCommand {
    pub(crate) fn into_parts(
        self,
        default_crawl_policy: CrawlPolicy,
    ) -> (SubscriptionKey, FeedSubscriptionAttrs) {
        let subscription = SubscriptionKey::new(self.subscriber_id, self.feed_url);
        let attrs = FeedSubscriptionAttrs {
            requirement: self.requirement,
            category: self.category,
            crawl_policy: self.crawl_policy.unwrap_or(default_crawl_policy),
        };
        (subscription, attrs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscribeFeedOutput {
    pub outcome: SubscribeOutcome,
}

impl SubscribeFeedOutput {
    pub fn subscription(&self) -> &SubscriptionKey {
        match &self.outcome {
            SubscribeOutcome::Subscribed(subscription)
            | SubscribeOutcome::Changed(subscription) => subscription,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnsubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}

impl UnsubscribeFeedCommand {
    pub(crate) fn into_subscription(self) -> SubscriptionKey {
        SubscriptionKey::new(self.subscriber_id, self.feed_url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsubscribeFeedOutput {
    pub outcome: UnsubscribeOutcome,
}

impl UnsubscribeFeedOutput {
    pub fn subscription(&self) -> &SubscriptionKey {
        match &self.outcome {
            UnsubscribeOutcome::Unsubscribed(subscription) => subscription,
        }
    }
}
