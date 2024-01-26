use async_graphql::{
    connection::{Connection, Edge},
    Context, Object, Result,
};

use crate::{
    gql::{
        object::{self, id, FeedEntry},
        run_usecase,
    },
    usecase::{
        FetchEntries, FetchEntriesInput, FetchEntriesOutput, FetchSubscribedFeeds,
        FetchSubscribedFeedsInput, FetchSubscribedFeedsOutput, Output,
    },
};

struct Subscription;

#[Object]
impl Subscription {
    async fn feeds(
        &self,
        cx: &Context<'_>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, object::Feed>> {
        let first = first.unwrap_or(10).min(100) as usize;
        let has_prev = after.is_some();
        let input = FetchSubscribedFeedsInput {
            after,
            first: first + 1,
        };
        let Output {
            output: FetchSubscribedFeedsOutput { feeds },
        } = run_usecase!(FetchSubscribedFeeds, cx, input)?;

        let has_next = feeds.len() > first;
        let mut connection = Connection::new(has_prev, has_next);

        let edges = feeds
            .into_iter()
            .map(|feed| (feed.meta().url().to_owned(), feed))
            .map(|(cursor, feed)| (cursor, object::Feed::from(feed)))
            .map(|(cursor, feed)| Edge::new(cursor, feed));

        connection.edges.extend(edges);

        Ok(connection)
    }
}

struct Feeds;

#[Object]
impl Feeds {
    /// Return Latest Feed entries
    async fn entries<'cx>(
        &self,
        cx: &Context<'_>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<id::EntryId, FeedEntry<'cx>>> {
        let first = first.unwrap_or(20).min(200) as usize;
        let has_prev = after.is_some();
        let input = FetchEntriesInput {
            after: after.map(Into::into),
            first: first + 1,
        };
        let Output {
            output: FetchEntriesOutput { entries, feeds },
        } = run_usecase!(FetchEntries, cx, input)?;

        let has_next = entries.len() > first;
        let mut connection = Connection::new(has_prev, has_next);

        let edges = entries.into_iter().map(move |(entry, feed_url)| {
            let meta = feeds
                .get(&feed_url)
                .expect("FeedMeta not found. this is a bug")
                .clone();
            let cursor = entry.id().into();
            let node = FeedEntry {
                feed: meta.into(),
                entry: entry.into(),
            };

            Edge::new(cursor, node)
        });

        connection.edges.extend(edges);

        Ok(connection)
    }
}

pub struct Query;

#[Object]
impl Query {
    async fn subscription(&self) -> Subscription {
        Subscription {}
    }

    async fn feeds(&self) -> Feeds {
        Feeds {}
    }
}
