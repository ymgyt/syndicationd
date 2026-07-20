use std::{collections::HashMap, sync::Arc};

use async_graphql::{
    Context, Enum, Object, Result, SimpleObject, Union,
    connection::{Connection, Edge, EmptyFields},
};
use synd_feed::types::{Annotated, Category, Feed, FeedUrl, Requirement};
use synd_registry::{
    Subscription as RegistrySubscription,
    crawl::policy::{CrawlPolicy as RegistryCrawlPolicy, PollingPolicy as RegistryPollingPolicy},
    query::{
        Subscriptions, SubscriptionsQuery, TimelineChange as RegistryTimelineChange,
        TimelineChangesQuery, TimelineEntriesPage, TimelineEntriesQuery,
        TimelineEntry as RegistryTimelineEntry, TimelineEntryCursor,
    },
};

use crate::gql::{
    object::{self, Entry},
    registry, subscriber_id,
};

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

    async fn feed(&self) -> Option<&object::Feed> {
        self.feed.as_ref()
    }
}

impl From<SubscribedFeed> for Edge<String, SubscribedFeed, EmptyFields> {
    fn from(feed: SubscribedFeed) -> Self {
        Self::new(feed.subscription.feed_url.to_string(), feed)
    }
}

/// Complete feeds resolved for one subscriptions page.
struct FeedBatch(HashMap<FeedUrl, Feed>);

impl FeedBatch {
    async fn load(cx: &Context<'_>, subscriptions: &Subscriptions) -> Result<Self> {
        let feed_urls = subscriptions
            .subscriptions
            .iter()
            .map(|subscription| subscription.feed_url.clone())
            .collect::<Vec<_>>();
        Ok(Self(registry(cx).load_feeds(&feed_urls).await?))
    }

    fn resolve(&mut self, subscription: RegistrySubscription) -> SubscribedFeed {
        let feed = self.0.remove(&subscription.feed_url);
        SubscribedFeed::resolve(subscription, feed)
    }
}

type SubscribedFeedsConnection = Connection<String, SubscribedFeed>;

/// GraphQL subscription nodes resolved against the latest observed feeds.
struct SubscribedFeedsPage {
    feeds: Vec<SubscribedFeed>,
    has_next_page: bool,
}

impl SubscribedFeedsPage {
    async fn load(cx: &Context<'_>, after: Option<String>, first: Option<i32>) -> Result<Self> {
        let page = registry(cx)
            .list_subscriptions(SubscriptionsQuery {
                subscriber_id: subscriber_id(cx),
                after,
                first: usize::try_from(first.unwrap_or(20).clamp(0, 100)).unwrap_or(0),
            })
            .await?;
        let feeds = FeedBatch::load(cx, &page).await?;
        Ok(Self::resolve(page, feeds))
    }

    fn resolve(page: Subscriptions, mut feeds: FeedBatch) -> Self {
        Self {
            feeds: page
                .subscriptions
                .into_iter()
                .map(|subscription| feeds.resolve(subscription))
                .collect(),
            has_next_page: page.has_next_page,
        }
    }
}

impl From<SubscribedFeedsPage> for SubscribedFeedsConnection {
    fn from(page: SubscribedFeedsPage) -> Self {
        let mut connection = Self::new(false, page.has_next_page);
        connection
            .edges
            .extend(page.feeds.into_iter().map(Edge::from));
        connection
    }
}

impl SubscribedFeed {
    fn resolve(subscription: RegistrySubscription, feed: Option<Feed>) -> Self {
        let feed = feed.map(|feed| {
            Annotated {
                feed: Arc::new(feed),
                requirement: subscription.requirement,
                category: subscription.category.clone(),
            }
            .into()
        });
        Self { subscription, feed }
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
        Ok(SubscribedFeedsPage::load(cx, after, first).await?.into())
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
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, TimelineEntry, TimelineEntriesFields>> {
        let request = TimelineEntriesRequest::from_graphql(cx, after.as_deref(), first)?;
        Ok(request.load(cx).await?.into())
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

/// Entry as it appears on one timeline: display position and content.
#[derive(SimpleObject)]
struct TimelineEntry {
    /// Display position on the timeline
    order_time: crate::gql::scalar::Rfc3339Time,
    entry: Entry,
}

impl From<RegistryTimelineEntry> for TimelineEntry {
    fn from(node: RegistryTimelineEntry) -> Self {
        let order_time = node.cursor.order_time().into();
        Self {
            order_time,
            entry: node.into(),
        }
    }
}

/// Connection-level fields of one timeline entries page.
#[derive(SimpleObject)]
struct TimelineEntriesFields {
    /// Change seq this page reflects. Clients sync changes from here
    seq: i64,
}

type TimelineEntriesConnection = Connection<String, TimelineEntry, TimelineEntriesFields>;

/// Registry timeline page at the GraphQL connection boundary.
struct TimelineEntriesGraphqlPage(TimelineEntriesPage);

impl From<TimelineEntriesGraphqlPage> for TimelineEntriesConnection {
    fn from(page: TimelineEntriesGraphqlPage) -> Self {
        let page = page.0;
        let mut connection = Self::with_additional_fields(
            false,
            page.has_next_page,
            TimelineEntriesFields { seq: page.seq },
        );
        connection.edges.extend(page.nodes.into_iter().map(|node| {
            let cursor = node.cursor.encode();
            Edge::new(cursor, TimelineEntry::from(node))
        }));
        connection
    }
}

/// Validated GraphQL arguments for one timeline entries query.
struct TimelineEntriesRequest(TimelineEntriesQuery);

impl TimelineEntriesRequest {
    fn from_graphql(cx: &Context<'_>, after: Option<&str>, first: Option<i32>) -> Result<Self> {
        let first = usize::try_from(first.unwrap_or(20).clamp(0, 100)).unwrap_or(0);
        let after = after
            .map(TimelineEntryCursor::decode)
            .transpose()
            .map_err(|err| async_graphql::Error::new(err.to_string()))?;
        Ok(Self(TimelineEntriesQuery {
            subscriber_id: subscriber_id(cx),
            after,
            first,
        }))
    }

    async fn load(self, cx: &Context<'_>) -> Result<TimelineEntriesGraphqlPage> {
        Ok(TimelineEntriesGraphqlPage(
            registry(cx).list_timeline_entries(self.0).await?,
        ))
    }
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
    timeline_entry: Box<TimelineEntry>,
}

#[derive(SimpleObject)]
struct TimelineChangeRemove {
    entry_id: String,
}

impl From<RegistryTimelineChange> for TimelineChange {
    fn from(change: RegistryTimelineChange) -> Self {
        match change {
            RegistryTimelineChange::Upsert(node) => Self::Upsert(TimelineChangeUpsert {
                timeline_entry: Box::new(TimelineEntry::from(*node)),
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
}
