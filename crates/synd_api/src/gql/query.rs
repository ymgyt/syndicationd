use std::borrow::Cow;

use async_graphql::{
    Context, Object, Result, SimpleObject,
    connection::{Connection, Edge},
};
use synd_feed::types::FeedUrl;

use crate::{
    gql::{
        object::{self, Entry, id},
        run_usecase,
    },
    usecase::{
        FetchEntries, FetchEntriesError, FetchEntriesInput, FetchEntriesOutput,
        FetchSubscribedFeeds, FetchSubscribedFeedsError, FetchSubscribedFeedsInput,
        FetchSubscribedFeedsOutput, Output,
    },
};

#[derive(SimpleObject)]
struct FeedsConnectionFields {
    errors: Vec<FetchFeedError>,
}

#[derive(SimpleObject)]
struct FetchFeedError {
    url: FeedUrl,
    error_message: String,
}

struct Subscription;

#[Object]
impl Subscription {
    /// Return Subscribed feeds
    async fn feeds(
        &self,
        cx: &Context<'_>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<String, object::Feed, FeedsConnectionFields>> {
        #[allow(clippy::cast_sign_loss)]
        let first = first.unwrap_or(10).min(100) as usize;
        let has_prev = after.is_some();
        let input = FetchSubscribedFeedsInput {
            after,
            first: first + 1,
        };
        let Output {
            output: FetchSubscribedFeedsOutput { feeds },
        } = run_usecase!(
            FetchSubscribedFeeds,
            cx,
            input,
            |err: FetchSubscribedFeedsError| Err(async_graphql::ErrorExtensions::extend(&err))
        )?;

        let has_next = feeds.len() > first;

        let (feeds, errors): (Vec<_>, Vec<_>) = feeds.into_iter().partition(Result::is_ok);
        let fields = FeedsConnectionFields {
            errors: errors
                .into_iter()
                .map(|err| {
                    let (url, fetch_err) = err.unwrap_err();
                    FetchFeedError {
                        url,
                        error_message: fetch_err.to_string(),
                    }
                })
                .collect::<Vec<_>>(),
        };
        let mut connection = Connection::with_additional_fields(has_prev, has_next, fields);

        let edges = feeds
            .into_iter()
            .take(first)
            .map(Result::unwrap)
            .map(|feed| (feed.feed.meta().url().as_str().to_owned(), feed))
            .map(|(cursor, feed)| (cursor, object::Feed::from(feed)))
            .map(|(cursor, feed)| Edge::new(cursor, feed));

        connection.edges.extend(edges);

        Ok(connection)
    }

    /// Return subscribed latest entries order by published time.
    async fn entries<'cx>(
        &'_ self,
        cx: &Context<'_>,
        after: Option<String>,
        #[graphql(default = 20)] first: Option<i32>,
    ) -> Result<Connection<id::EntryId<'_>, Entry<'cx>>> {
        #[allow(clippy::cast_sign_loss)]
        let first = first.unwrap_or(20).min(200) as usize;
        let has_prev = after.is_some();
        let input = FetchEntriesInput {
            after: after.map(Into::into),
            first: first + 1,
        };
        let Output {
            output: FetchEntriesOutput { entries, feeds },
        } = run_usecase!(FetchEntries, cx, input, |err: FetchEntriesError| Err(
            async_graphql::ErrorExtensions::extend(&err)
        ))?;

        let has_next = entries.len() > first;
        let mut connection = Connection::new(has_prev, has_next);

        let edges = entries
            .into_iter()
            .take(first)
            .map(move |(entry, feed_url)| {
                let meta = feeds
                    .get(&feed_url)
                    .expect("FeedMeta not found. this is a bug")
                    .clone();
                let cursor = entry.id().into();
                let node = Entry::new(Cow::Owned(meta), entry);
                Edge::new(cursor, node)
            });

        connection.edges.extend(edges);

        Ok(connection)
    }
}

pub(crate) struct Query;

#[Object]
impl Query {
    async fn subscription(&self) -> Subscription {
        Subscription {}
    }
}
