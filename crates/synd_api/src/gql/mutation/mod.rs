use async_graphql::{Context, Enum, Error, InputObject, Object, SimpleObject};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    SubscribeFeedCommand, SubscribeOutcome, UnsubscribeFeedCommand, UnsubscribeOutcome,
    crawl::policy::{CrawlPolicy, PollingInterval, PollingPolicy},
};

use crate::gql::{registry, subscriber_id};

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ResponseCode {
    Ok,
    Unauthorized,
    InvalidFeedUrl,
    FeedUnavailable,
    InternalError,
}

#[derive(SimpleObject, Clone, PartialEq, Eq, Debug)]
pub(crate) struct ResponseStatus {
    code: ResponseCode,
}

impl ResponseStatus {
    pub(crate) fn ok() -> Self {
        ResponseStatus {
            code: ResponseCode::Ok,
        }
    }
}

#[derive(InputObject)]
struct SubscribeFeedInput {
    url: FeedUrl,
    requirement: Option<Requirement>,
    category: Option<Category<'static>>,
    crawl_policy: Option<CrawlPolicyInput>,
}

#[derive(InputObject)]
struct CrawlPolicyInput {
    polling: PollingPolicyInput,
}

impl CrawlPolicyInput {
    fn into_policy(self) -> async_graphql::Result<CrawlPolicy> {
        Ok(CrawlPolicy {
            polling: self.polling.into_policy()?,
        })
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum PollingPolicyKindInput {
    Manual,
    Interval,
}

#[derive(InputObject)]
struct PollingPolicyInput {
    kind: PollingPolicyKindInput,
    interval_seconds: Option<i64>,
}

impl PollingPolicyInput {
    fn into_policy(self) -> async_graphql::Result<PollingPolicy> {
        match self.kind {
            PollingPolicyKindInput::Manual => {
                if self.interval_seconds.is_some() {
                    return Err(Error::new(
                        "intervalSeconds must be omitted when polling policy kind is MANUAL",
                    ));
                }
                Ok(PollingPolicy::manual())
            }
            PollingPolicyKindInput::Interval => {
                let seconds = self.interval_seconds.ok_or_else(|| {
                    Error::new("intervalSeconds is required when polling policy kind is INTERVAL")
                })?;
                let seconds = u64::try_from(seconds)
                    .map_err(|_| Error::new("intervalSeconds must be a positive integer"))?;
                if seconds == 0 {
                    return Err(Error::new("intervalSeconds must be greater than zero"));
                }
                let interval = PollingInterval::try_from(std::time::Duration::from_secs(seconds))
                    .map_err(|err| Error::new(err.to_string()))?;
                Ok(PollingPolicy::interval(interval))
            }
        }
    }
}

#[derive(SimpleObject)]
struct SubscribeFeedPayload {
    status: ResponseStatus,
    url: FeedUrl,
    disposition: SubscribeDisposition,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum SubscribeDisposition {
    Subscribed,
    Changed,
}

#[derive(InputObject)]
struct UnsubscribeFeedInput {
    url: FeedUrl,
}

#[derive(SimpleObject)]
struct UnsubscribeFeedPayload {
    status: ResponseStatus,
    url: FeedUrl,
    disposition: UnsubscribeDisposition,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum UnsubscribeDisposition {
    Unsubscribed,
}

pub(crate) struct Mutation;

#[Object]
impl Mutation {
    async fn subscribe_feed(
        &self,
        cx: &Context<'_>,
        input: SubscribeFeedInput,
    ) -> async_graphql::Result<SubscribeFeedPayload> {
        let subscriber_id = subscriber_id(cx);
        let crawl_policy = input
            .crawl_policy
            .map(CrawlPolicyInput::into_policy)
            .transpose()?;
        let out = registry(cx)
            .subscribe(SubscribeFeedCommand {
                subscriber_id,
                feed_url: input.url.clone(),
                requirement: input.requirement,
                category: input.category,
                crawl_policy,
            })
            .await?;

        let (url, disposition) = match out.outcome {
            SubscribeOutcome::Subscribed(subscription) => {
                (subscription.feed_url, SubscribeDisposition::Subscribed)
            }
            SubscribeOutcome::Changed(subscription) => {
                (subscription.feed_url, SubscribeDisposition::Changed)
            }
        };

        Ok(SubscribeFeedPayload {
            status: ResponseStatus::ok(),
            url,
            disposition,
        })
    }

    async fn unsubscribe_feed(
        &self,
        cx: &Context<'_>,
        input: UnsubscribeFeedInput,
    ) -> async_graphql::Result<UnsubscribeFeedPayload> {
        let out = registry(cx)
            .unsubscribe(UnsubscribeFeedCommand {
                subscriber_id: subscriber_id(cx),
                feed_url: input.url,
            })
            .await?;
        let (url, disposition) = match out.outcome {
            UnsubscribeOutcome::Unsubscribed(subscription) => {
                (subscription.feed_url, UnsubscribeDisposition::Unsubscribed)
            }
        };

        Ok(UnsubscribeFeedPayload {
            status: ResponseStatus::ok(),
            url,
            disposition,
        })
    }

}
