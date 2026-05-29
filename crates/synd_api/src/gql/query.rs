use std::{borrow::Cow, sync::Arc};

use async_graphql::{
    Context, Enum, Object, Result, SimpleObject,
    connection::{Connection, Edge},
};
use synd_feed::types::{Annotated, Category, FeedUrl, Requirement};
use synd_registry::legacy::model::{
    EntryCursor, FeedStatusQuery, FeedSubscription, FeedSubscriptionView, ListEntriesQuery,
    ListSubscriptionsQuery, RefreshPolicy as RegistryRefreshPolicy, RefreshSchedule,
    RefreshStatus as RegistryRefreshStatus, RefreshStatusKind,
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

impl From<RefreshStatusKind> for RefreshStatusState {
    fn from(value: RefreshStatusKind) -> Self {
        match value {
            RefreshStatusKind::NeverRefreshed => Self::NeverRefreshed,
            RefreshStatusKind::Idle => Self::Idle,
            RefreshStatusKind::Pending => Self::Pending,
            RefreshStatusKind::Running => Self::Running,
            RefreshStatusKind::LastFailed => Self::LastFailed,
        }
    }
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

impl From<RegistryRefreshStatus> for RefreshStatus {
    fn from(value: RegistryRefreshStatus) -> Self {
        Self {
            state: value.kind.into(),
            request_id: value.active_request_id.map(|id| id.to_string()),
            last_attempt_at: value.last_attempt_at.map(Into::into),
            last_success_at: value.last_success_at.map(Into::into),
            last_failure_at: value.last_failure_at.map(Into::into),
            last_error_message: value.last_error_message,
        }
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum RefreshPolicyKind {
    Manual,
    Interval,
}

#[derive(SimpleObject)]
struct RefreshPolicy {
    kind: RefreshPolicyKind,
    interval_seconds: Option<i64>,
}

impl From<RegistryRefreshPolicy> for RefreshPolicy {
    fn from(value: RegistryRefreshPolicy) -> Self {
        match value.schedule {
            RefreshSchedule::Manual => Self {
                kind: RefreshPolicyKind::Manual,
                interval_seconds: None,
            },
            RefreshSchedule::Interval(duration) => Self {
                kind: RefreshPolicyKind::Interval,
                interval_seconds: Some(i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)),
            },
        }
    }
}

struct SubscribedFeed {
    subscription: FeedSubscription,
    feed: Option<object::Feed>,
    refresh_status: RefreshStatus,
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

    async fn refresh_policy(&self) -> RefreshPolicy {
        self.subscription.refresh_policy.into()
    }

    async fn refresh_status(&self) -> &RefreshStatus {
        &self.refresh_status
    }

    async fn feed(&self) -> Option<&object::Feed> {
        self.feed.as_ref()
    }
}

impl From<FeedSubscriptionView> for SubscribedFeed {
    fn from(value: FeedSubscriptionView) -> Self {
        let annotations = Annotated {
            feed: (),
            requirement: value.subscription.requirement,
            category: value.subscription.category.clone(),
        };
        let feed = value.feed.map(|feed| {
            object::Feed::from(Annotated {
                feed: Arc::new(feed),
                requirement: annotations.requirement,
                category: annotations.category,
            })
        });

        Self {
            subscription: value.subscription,
            feed,
            refresh_status: value.refresh_status.into(),
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

    async fn feed_status(&self, cx: &Context<'_>, url: FeedUrl) -> Result<RefreshStatus> {
        registry(cx)
            .feed_status(FeedStatusQuery {
                subscriber_id: subscriber_id(cx),
                feed_url: url,
            })
            .await
            .map(Into::into)
            .map_err(Into::into)
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
        .list_subscriptions(ListSubscriptionsQuery {
            subscriber_id: subscriber_id(cx),
            after,
            first,
        })
        .await?;

    let mut connection = Connection::new(false, page.has_next_page);
    connection.edges.extend(
        page.nodes
            .into_iter()
            .take(first)
            .map(SubscribedFeed::from)
            .map(|feed| Edge::new(feed.subscription.feed_url.to_string(), feed)),
    );

    Ok(connection)
}

async fn timeline_entries_connection<'cx>(
    cx: &Context<'cx>,
    after: Option<String>,
    first: Option<i32>,
) -> Result<Connection<String, Entry<'cx>>> {
    let first = usize::try_from(first.unwrap_or(20).clamp(0, 200)).unwrap_or(0);
    let after = after
        .as_deref()
        .map(EntryCursor::decode)
        .transpose()
        .map_err(|err| async_graphql::Error::new(err.to_string()))?;
    let page = registry(cx)
        .list_entries(ListEntriesQuery {
            subscriber_id: subscriber_id(cx),
            after,
            first,
        })
        .await?;

    let mut connection = Connection::new(false, page.has_next_page);
    connection
        .edges
        .extend(page.nodes.into_iter().take(first).map(|view| {
            let cursor = view.cursor.encode();
            Edge::new(cursor, Entry::new(Cow::Owned(view.feed_meta), view.entry))
        }));

    Ok(connection)
}
