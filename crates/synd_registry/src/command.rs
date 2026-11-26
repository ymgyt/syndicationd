use synd_feed::types::{FeedUrl, SubscriptionLabels, TimelineName};

use crate::{
    crawl::{policy::CrawlPolicy, request::RequestCrawlOutcome},
    subscription::{
        FeedSubscriptionAttrs, SubscribeOutcome, SubscriberId, SubscriptionKey, UnsubscribeOutcome,
    },
    timeline::TimelineId,
};

/// Request to create a named timeline owned by the subscriber.
#[derive(Debug, Clone)]
pub struct CreateTimelineCommand {
    pub subscriber_id: SubscriberId,
    pub name: TimelineName,
}

/// Identity of the timeline saved by the create command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTimelineOutput {
    pub timeline_id: TimelineId,
}

/// Request to create or update one subscriber/feed relation.
#[derive(Debug, Clone)]
pub struct SubscribeFeedCommand {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub labels: SubscriptionLabels,
    pub crawl_policy: Option<CrawlPolicy>,
}

impl SubscribeFeedCommand {
    pub(crate) fn into_parts(
        self,
        default_crawl_policy: CrawlPolicy,
    ) -> (SubscriptionKey, FeedSubscriptionAttrs) {
        let subscription = SubscriptionKey::new(self.subscriber_id, self.feed_url);
        let attrs = FeedSubscriptionAttrs {
            labels: self.labels,
            crawl_policy: self.crawl_policy.unwrap_or(default_crawl_policy),
        };
        (subscription, attrs)
    }
}

/// Result returned after handling a subscribe request.
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

/// Request to end one subscriber/feed relation.
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

/// Result returned after handling an unsubscribe request.
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

/// Request to trigger one crawl outside the periodic schedule.
#[derive(Debug, Clone)]
pub struct RequestCrawlCommand {
    pub feed_url: FeedUrl,
}

/// Result returned after handling a crawl request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCrawlOutput {
    pub outcome: RequestCrawlOutcome,
}
