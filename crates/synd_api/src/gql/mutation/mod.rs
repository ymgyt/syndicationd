use async_graphql::{Context, Enum, Error, InputObject, Object, SimpleObject};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::legacy::model::{
    InitialRefreshMode, RefreshInterval, RefreshPolicy, RefreshRequestDisposition, RefreshSchedule,
    RequestRefreshCommand, SubscribeFeedCommand, SubscribeFeedRefresh, UnsubscribeFeedCommand,
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

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum InitialRefreshModeInput {
    Async,
    RequireSuccess,
}

impl From<InitialRefreshModeInput> for InitialRefreshMode {
    fn from(value: InitialRefreshModeInput) -> Self {
        match value {
            InitialRefreshModeInput::Async => Self::Async,
            InitialRefreshModeInput::RequireSuccess => Self::RequireSuccess,
        }
    }
}

#[derive(InputObject)]
struct SubscribeFeedInput {
    url: FeedUrl,
    requirement: Option<Requirement>,
    category: Option<Category<'static>>,
    refresh_policy: Option<RefreshPolicyInput>,
    initial_refresh: Option<InitialRefreshModeInput>,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum RefreshPolicyKindInput {
    Manual,
    Interval,
}

#[derive(InputObject)]
struct RefreshPolicyInput {
    kind: RefreshPolicyKindInput,
    interval_seconds: Option<i64>,
}

impl RefreshPolicyInput {
    fn into_policy(self) -> async_graphql::Result<RefreshPolicy> {
        match self.kind {
            RefreshPolicyKindInput::Manual => {
                if self.interval_seconds.is_some() {
                    return Err(Error::new(
                        "intervalSeconds must be omitted when refresh policy kind is MANUAL",
                    ));
                }
                Ok(RefreshPolicy {
                    schedule: RefreshSchedule::Manual,
                })
            }
            RefreshPolicyKindInput::Interval => {
                let seconds = self.interval_seconds.ok_or_else(|| {
                    Error::new("intervalSeconds is required when refresh policy kind is INTERVAL")
                })?;
                let seconds = u64::try_from(seconds)
                    .map_err(|_| Error::new("intervalSeconds must be a positive integer"))?;
                if seconds == 0 {
                    return Err(Error::new("intervalSeconds must be greater than zero"));
                }
                let interval = RefreshInterval::try_from(std::time::Duration::from_secs(seconds))
                    .map_err(|err| Error::new(err.to_string()))?;
                Ok(RefreshPolicy {
                    schedule: RefreshSchedule::Interval(interval),
                })
            }
        }
    }
}

#[derive(SimpleObject)]
struct SubscribeFeedPayload {
    status: ResponseStatus,
    url: FeedUrl,
    request_id: Option<String>,
    disposition: Option<RefreshDisposition>,
}

#[derive(InputObject)]
struct UnsubscribeFeedInput {
    url: FeedUrl,
}

#[derive(SimpleObject)]
struct UnsubscribeFeedPayload {
    status: ResponseStatus,
}

#[derive(InputObject)]
struct RefreshFeedInput {
    url: FeedUrl,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum RefreshDisposition {
    Created,
    Promoted,
    CoalescedPending,
    JoinedRunning,
}

impl From<RefreshRequestDisposition> for RefreshDisposition {
    fn from(value: RefreshRequestDisposition) -> Self {
        match value {
            RefreshRequestDisposition::Created => Self::Created,
            RefreshRequestDisposition::Promoted => Self::Promoted,
            RefreshRequestDisposition::CoalescedPending => Self::CoalescedPending,
            RefreshRequestDisposition::JoinedRunning => Self::JoinedRunning,
        }
    }
}

#[derive(SimpleObject)]
struct RefreshFeedPayload {
    status: ResponseStatus,
    request_id: String,
    disposition: RefreshDisposition,
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
        let refresh_policy = match input.refresh_policy {
            Some(policy) => policy.into_policy()?,
            None => registry(cx).default_refresh_policy(),
        };
        let out = registry(cx)
            .subscribe(SubscribeFeedCommand {
                subscriber_id,
                feed_url: input.url.clone(),
                requirement: input.requirement,
                category: input.category,
                refresh_policy,
                initial_refresh: input
                    .initial_refresh
                    .unwrap_or(InitialRefreshModeInput::Async)
                    .into(),
            })
            .await?;

        let (request_id, disposition) = match out.refresh {
            SubscribeFeedRefresh::Enqueued(refresh) => (
                Some(refresh.request_id.to_string()),
                Some(refresh.disposition.into()),
            ),
            SubscribeFeedRefresh::Completed(_) => (None, None),
        };

        Ok(SubscribeFeedPayload {
            status: ResponseStatus::ok(),
            url: out.subscription.feed_url,
            request_id,
            disposition,
        })
    }

    async fn unsubscribe_feed(
        &self,
        cx: &Context<'_>,
        input: UnsubscribeFeedInput,
    ) -> async_graphql::Result<UnsubscribeFeedPayload> {
        registry(cx)
            .unsubscribe(UnsubscribeFeedCommand {
                subscriber_id: subscriber_id(cx),
                feed_url: input.url,
            })
            .await?;

        Ok(UnsubscribeFeedPayload {
            status: ResponseStatus::ok(),
        })
    }

    async fn refresh_feed(
        &self,
        cx: &Context<'_>,
        input: RefreshFeedInput,
    ) -> async_graphql::Result<RefreshFeedPayload> {
        let out = registry(cx)
            .request_refresh(RequestRefreshCommand {
                subscriber_id: subscriber_id(cx),
                feed_url: input.url,
            })
            .await?;

        Ok(RefreshFeedPayload {
            status: ResponseStatus::ok(),
            request_id: out.request_id.to_string(),
            disposition: out.disposition.into(),
        })
    }
}
