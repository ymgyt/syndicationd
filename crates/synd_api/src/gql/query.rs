use async_graphql::{
    Context, Enum, Object, Result, SimpleObject,
    connection::{Connection, Edge},
};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    Subscription as RegistrySubscription,
    crawl::policy::{CrawlPolicy as RegistryCrawlPolicy, PollingPolicy as RegistryPollingPolicy},
    query::SubscriptionsQuery,
};

use crate::gql::{
    object::{self, Entry},
    registry, subscriber_id,
};

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum RefreshStatusState {
    NeverRefreshed,
    Idle,
    Pending,
    Running,
    LastFailed,
}

#[derive(SimpleObject)]
struct RefreshStatus {
    state: RefreshStatusState,
    request_id: Option<String>,
    last_attempt_at: Option<crate::gql::scalar::Rfc3339Time>,
    last_success_at: Option<crate::gql::scalar::Rfc3339Time>,
    last_failure_at: Option<crate::gql::scalar::Rfc3339Time>,
    last_error_message: Option<String>,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum PollingPolicyKind {
    Manual,
    Interval,
}

#[derive(SimpleObject)]
struct PollingPolicy {
    kind: PollingPolicyKind,
    interval_seconds: Option<i64>,
}

impl From<RegistryPollingPolicy> for PollingPolicy {
    fn from(value: RegistryPollingPolicy) -> Self {
        match value {
            RegistryPollingPolicy::Manual => Self {
                kind: PollingPolicyKind::Manual,
                interval_seconds: None,
            },
            RegistryPollingPolicy::Interval { interval } => Self {
                kind: PollingPolicyKind::Interval,
                interval_seconds: Some(i64::try_from(interval.as_secs()).unwrap_or(i64::MAX)),
            },
        }
    }
}

#[derive(SimpleObject)]
struct CrawlPolicy {
    polling: PollingPolicy,
}

impl From<RegistryCrawlPolicy> for CrawlPolicy {
    fn from(value: RegistryCrawlPolicy) -> Self {
        Self {
            polling: value.polling.into(),
        }
    }
}

struct SubscribedFeed {
    subscription: RegistrySubscription,
    feed: Option<object::Feed>,
    refresh_status: Option<RefreshStatus>,
}

#[Object]
impl SubscribedFeed {
    async fn url(&self) -> &FeedUrl {
        &self.subscription.feed_url
    }

    async fn requirement(&self) -> Option<Requirement> {
        self.subscription.requirement
    }

    async fn category(&self) -> Option<&Category<'static>> {
        self.subscription.category.as_ref()
    }

    async fn crawl_policy(&self) -> CrawlPolicy {
        self.subscription.crawl_policy.into()
    }

    async fn refresh_status(&self) -> Option<&RefreshStatus> {
        self.refresh_status.as_ref()
    }

    async fn feed(&self) -> Option<&object::Feed> {
        self.feed.as_ref()
    }
}

impl From<RegistrySubscription> for SubscribedFeed {
    fn from(subscription: RegistrySubscription) -> Self {
        Self {
            subscription,
            feed: None,
            refresh_status: None,
        }
    }
}

struct Subscription;

#[Object]
impl Subscription {
    async fn feeds(
        &self,
        cx: &Context<'_>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, SubscribedFeed>> {
        subscriptions_connection(cx, after, first).await
    }

    async fn entries<'cx>(
        &'_ self,
        cx: &'cx Context<'cx>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, Entry<'cx>>> {
        timeline_entries_connection(cx, after, first).await
    }

    #[expect(clippy::unused_async)]
    async fn feed_status(&self, cx: &Context<'_>, url: FeedUrl) -> Result<RefreshStatus> {
        let _ = (cx, url);
        Err(async_graphql::Error::new(
            "feedStatus is not implemented while crawl runtime is redesigned",
        ))
    }
}

struct FeedRegistry;

#[Object]
impl FeedRegistry {
    async fn subscriptions(
        &self,
        cx: &Context<'_>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, SubscribedFeed>> {
        subscriptions_connection(cx, after, first).await
    }

    async fn timeline(&self) -> Timeline {
        Timeline
    }
}

struct Timeline;

#[Object]
impl Timeline {
    async fn entries<'cx>(
        &'_ self,
        cx: &'cx Context<'cx>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, Entry<'cx>>> {
        timeline_entries_connection(cx, after, first).await
    }
}

pub(crate) struct Query;

#[Object]
impl Query {
    async fn feed_registry(&self) -> FeedRegistry {
        FeedRegistry
    }

    async fn subscription(&self) -> Subscription {
        Subscription {}
    }
}

async fn subscriptions_connection(
    cx: &Context<'_>,
    after: Option<String>,
    first: Option<i32>,
) -> Result<Connection<String, SubscribedFeed>> {
    let first = usize::try_from(first.unwrap_or(20).clamp(0, 100)).unwrap_or(0);
    let page = registry(cx)
        .list_subscriptions(SubscriptionsQuery {
            subscriber_id: subscriber_id(cx),
            after,
            first,
        })
        .await?;

    let mut connection = Connection::new(false, page.has_next_page);
    connection.edges.extend(
        page.subscriptions
            .into_iter()
            .take(first)
            .map(SubscribedFeed::from)
            .map(|feed| Edge::new(feed.subscription.feed_url.to_string(), feed)),
    );

    Ok(connection)
}

#[expect(clippy::unused_async)]
async fn timeline_entries_connection<'cx>(
    cx: &Context<'cx>,
    after: Option<String>,
    first: Option<i32>,
) -> Result<Connection<String, Entry<'cx>>> {
    let _ = (cx, after, first);
    Err(async_graphql::Error::new(
        "timeline entries are not implemented while timeline projection is redesigned",
    ))
}
