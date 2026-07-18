use async_graphql::{
    Context, Enum, Object, Result, SimpleObject, Union,
    connection::{Connection, Edge},
};
use synd_feed::types::{Category, FeedUrl, Requirement};
use synd_registry::{
    Subscription as RegistrySubscription,
    crawl::policy::{CrawlPolicy as RegistryCrawlPolicy, PollingPolicy as RegistryPollingPolicy},
    query::{
        SubscriptionsQuery, TimelineChange as RegistryTimelineChange, TimelineChangesQuery,
        TimelineItemCursor, TimelineItemsQuery,
    },
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

    async fn entries(
        &self,
        cx: &Context<'_>,
        url: Option<FeedUrl>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, Entry, TimelineEntriesFields>> {
        timeline_entries_connection(cx, url, after, first).await
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
    async fn entries(
        &self,
        cx: &Context<'_>,
        url: Option<FeedUrl>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, Entry, TimelineEntriesFields>> {
        timeline_entries_connection(cx, url, after, first).await
    }

    async fn changes(
        &self,
        cx: &Context<'_>,
        since: i64,
        #[graphql(default = 100)] first: Option<i32>,
    ) -> Result<TimelineChanges> {
        let limit = usize::try_from(first.unwrap_or(100).clamp(0, 500)).unwrap_or(0);
        let page = registry(cx)
            .list_timeline_changes(TimelineChangesQuery {
                subscriber_id: subscriber_id(cx),
                since,
                limit,
            })
            .await?;

        Ok(TimelineChanges {
            changes: page.changes.into_iter().map(Into::into).collect(),
            seq: page.seq,
            has_more: page.has_more,
        })
    }
}

/// Connection-level fields of one timeline entries page.
#[derive(SimpleObject)]
struct TimelineEntriesFields {
    /// Change seq this page reflects. Clients sync changes from here
    seq: i64,
}

/// Page of timeline changes for incremental sync, ordered by seq.
#[derive(SimpleObject)]
struct TimelineChanges {
    changes: Vec<TimelineChange>,
    /// Seq the client remembers after applying this page
    seq: i64,
    has_more: bool,
}

#[derive(Union)]
enum TimelineChange {
    Upsert(TimelineChangeUpsert),
    Remove(TimelineChangeRemove),
}

#[derive(SimpleObject)]
struct TimelineChangeUpsert {
    /// Display position of the entry
    order_time: crate::gql::scalar::Rfc3339Time,
    entry: Box<Entry>,
}

#[derive(SimpleObject)]
struct TimelineChangeRemove {
    entry_id: String,
}

impl From<RegistryTimelineChange> for TimelineChange {
    fn from(change: RegistryTimelineChange) -> Self {
        match change {
            RegistryTimelineChange::Upsert(node) => Self::Upsert(TimelineChangeUpsert {
                order_time: node.cursor.order_time().into(),
                entry: Box::new(Entry::from_timeline_item_node(*node)),
            }),
            RegistryTimelineChange::Remove { entry_id } => Self::Remove(TimelineChangeRemove {
                entry_id: entry_id.as_str().to_owned(),
            }),
        }
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

async fn timeline_entries_connection(
    cx: &Context<'_>,
    feed_url: Option<FeedUrl>,
    after: Option<String>,
    first: Option<i32>,
) -> Result<Connection<String, Entry, TimelineEntriesFields>> {
    let first = usize::try_from(first.unwrap_or(20).clamp(0, 100)).unwrap_or(0);
    let after = after
        .as_deref()
        .map(TimelineItemCursor::decode)
        .transpose()
        .map_err(|err| async_graphql::Error::new(err.to_string()))?;
    let page = registry(cx)
        .list_timeline_items(TimelineItemsQuery {
            subscriber_id: subscriber_id(cx),
            feed_url,
            after,
            first,
        })
        .await?;

    let mut connection = Connection::with_additional_fields(
        false,
        page.has_next_page,
        TimelineEntriesFields { seq: page.seq },
    );
    connection.edges.extend(page.nodes.into_iter().map(|node| {
        let cursor = node.cursor.encode();
        Edge::new(cursor, Entry::from_timeline_item_node(node))
    }));

    Ok(connection)
}
