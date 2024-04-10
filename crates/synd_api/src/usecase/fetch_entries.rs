use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use futures_util::{stream::FuturesUnordered, StreamExt};
use synd_feed::{
    feed::{cache::FetchCachedFeed, parser::FetchFeedError},
    types::{self, EntryId},
};
use thiserror::Error;

use crate::{
    principal::Principal,
    repository::SubscriptionRepository,
    usecase::{authorize::Unauthorized, Error, Input, MakeUsecase, Output, Usecase},
};

pub struct FetchEntries {
    pub repository: Arc<dyn SubscriptionRepository>,
    pub fetch_feed: Arc<dyn FetchCachedFeed>,
}

pub struct FetchEntriesInput {
    pub after: Option<EntryId<'static>>,
    pub first: usize,
}

#[derive(Default)]
pub struct FetchEntriesOutput {
    pub entries: Vec<(types::Entry, types::FeedUrl)>,
    pub feeds: HashMap<types::FeedUrl, types::FeedMeta>,
}

#[derive(Error, Debug)]
pub enum FetchEntriesError {}

impl Usecase for FetchEntries {
    type Input = FetchEntriesInput;

    type Output = FetchEntriesOutput;

    type Error = FetchEntriesError;

    fn new(make: &MakeUsecase) -> Self {
        Self {
            repository: make.subscription_repo.clone(),
            fetch_feed: make.fetch_feed.clone(),
        }
    }

    async fn authorize(
        &self,
        principal: Principal,
        _: &Self::Input,
    ) -> Result<Principal, Unauthorized> {
        Ok(principal)
    }

    #[tracing::instrument(name = "fetch_entries", skip(self, principal))]
    async fn usecase(
        &self,
        Input {
            principal,
            input: FetchEntriesInput { after, first },
        }: Input<Self::Input>,
    ) -> Result<Output<Self::Output>, Error<Self::Error>> {
        let user_id = principal
            .user_id()
            .expect("user id not found. this is a bug");

        let feeds = self.repository.fetch_subscribed_feeds(user_id).await?;

        // TODO: refactor
        let mut feed_metas = HashMap::with_capacity(feeds.urls.len());
        let mut entries = Vec::with_capacity(feeds.urls.len() * 4);
        let mut handle_feed = |feed: Result<Arc<types::Feed>, FetchFeedError>| {
            let feed = match feed {
                Ok(feed) => feed,
                Err(err) => {
                    tracing::warn!("Failed to fetch feed {err:?}");
                    return;
                }
            };

            let meta = feed.meta().clone();
            let feed_url = meta.url().to_owned();
            feed_metas.insert(feed_url.clone(), meta);
            entries.extend(
                feed.entries()
                    .cloned()
                    .map(|entry| (entry, feed_url.clone())),
            );
        };

        let mut tasks = FuturesUnordered::new();
        let in_flight_limit = 10;

        for url in feeds.urls {
            if tasks.len() >= in_flight_limit {
                if let Some(result) = tasks.next().await {
                    handle_feed(result);
                }
            }

            let fetch_feed = Arc::clone(&self.fetch_feed);
            tasks.push(async move { fetch_feed.fetch_feed(url.clone()).await });
        }

        while let Some(result) = tasks.next().await {
            handle_feed(result);
        }

        // Sort by published or updated
        entries.sort_unstable_by(|(a, _), (b, _)| {
            match (a.published().or(a.updated()), b.published().or(b.updated())) {
                (Some(a), Some(b)) => b.cmp(&a),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        // Paginate
        let entries = {
            let start = after
                .and_then(|after| {
                    entries
                        .iter()
                        .position(|(entry, _)| entry.id_ref() == after)
                        .map(|position| position + 1)
                })
                .unwrap_or(0);

            if start >= entries.len() {
                return Ok(Output {
                    output: Self::Output::default(),
                });
            }
            let mut entries = entries.split_off(start);
            entries.truncate(first);
            entries
        };

        Ok(Output {
            output: FetchEntriesOutput {
                entries,
                feeds: feed_metas,
            },
        })
    }
}
