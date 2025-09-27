use std::{cmp::Ordering, collections::HashMap, sync::Arc};

use futures_util::{StreamExt, stream::FuturesUnordered};
use synd_feed::{
    feed::{cache::FetchCachedFeed, service::FetchFeedError},
    types::{self, Annotated, Entry, EntryId, FeedMeta, FeedUrl},
};
use thiserror::Error;

use crate::{
    principal::Principal,
    repository::{
        SubscriptionRepository,
        types::{FeedAnnotations, SubscribedFeeds},
    },
    usecase::{Error, Input, MakeUsecase, Output, Usecase, authorize::Unauthorized},
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
    pub feeds: HashMap<types::FeedUrl, Annotated<types::FeedMeta>>,
}

#[derive(Error, Debug)]
#[error("fetch entries error")]
pub struct FetchEntriesError {}

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

        let SubscribedFeeds { urls, annotations } =
            self.repository.fetch_subscribed_feeds(user_id).await?;

        let output = self
            .operation(urls, annotations)
            .fetch()
            .await
            .sort()
            .paginate(first, after);

        Ok(output)
    }
}

impl FetchEntries {
    fn operation(
        &self,
        urls: Vec<FeedUrl>,
        annotations: Option<HashMap<FeedUrl, FeedAnnotations>>,
    ) -> FetchOperation {
        let len = urls.len();
        FetchOperation {
            urls: Some(urls),
            metas: HashMap::with_capacity(len),
            entries: Vec::with_capacity(len * 5),
            annotations,
            fetch_feed: self.fetch_feed.clone(),
        }
    }
}

struct FetchOperation {
    // urls to fetch. wrap `Option` for take ownership
    urls: Option<Vec<FeedUrl>>,
    // feed annotations got from repository
    annotations: Option<HashMap<FeedUrl, FeedAnnotations>>,
    // fetch service
    fetch_feed: Arc<dyn FetchCachedFeed>,

    // output
    metas: HashMap<FeedUrl, Annotated<FeedMeta>>,
    entries: Vec<(Entry, FeedUrl)>,
}

impl FetchOperation {
    // fetch given urls respecting concurrency limit
    async fn fetch(mut self) -> Self {
        let mut tasks = FuturesUnordered::new();
        let in_flight_limit = 10;

        for url in self.urls.take().unwrap() {
            if tasks.len() >= in_flight_limit
                && let Some(result) = tasks.next().await
            {
                self.handle(result);
            }

            let fetch_feed = Arc::clone(&self.fetch_feed);
            tasks.push(async move { fetch_feed.fetch_feed(url).await });
        }

        while let Some(result) = tasks.next().await {
            self.handle(result);
        }
        self
    }

    // handle fetch feed result
    fn handle(&mut self, feed: Result<Arc<types::Feed>, FetchFeedError>) {
        let feed = match feed {
            Ok(feed) => feed,
            Err(err) => {
                tracing::warn!("Failed to fetch feed {err:?}");
                return;
            }
        };

        let meta = feed.meta().clone();
        let feed_url = meta.url().to_owned();
        let meta = match self
            .annotations
            .as_mut()
            .and_then(|annotations| annotations.remove(&feed_url))
        {
            Some(feed_annotations) => Annotated {
                feed: meta,
                requirement: feed_annotations.requirement,
                category: feed_annotations.category,
            },
            None => Annotated::new(meta),
        };
        self.metas.insert(feed_url.clone(), meta);
        self.entries.extend(
            feed.entries()
                .cloned()
                .map(|entry| (entry, feed_url.clone())),
        );
    }

    // sort entries
    fn sort(mut self) -> Self {
        self.entries.sort_unstable_by(|(a, _), (b, _)| {
            match (a.published().or(a.updated()), b.published().or(b.updated())) {
                (Some(a), Some(b)) => b.cmp(&a),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        self
    }

    // paginate entries and return output
    fn paginate(
        mut self,
        first: usize,
        after: Option<EntryId<'static>>,
    ) -> Output<FetchEntriesOutput> {
        let start = after
            .and_then(|after| {
                self.entries
                    .iter()
                    .position(|(entry, _)| entry.id_ref() == after)
                    .map(|position| position + 1)
            })
            .unwrap_or(0);

        if start >= self.entries.len() {
            return Output {
                output: FetchEntriesOutput::default(),
            };
        }
        let mut entries = self.entries.split_off(start);
        entries.truncate(first);

        Output {
            output: FetchEntriesOutput {
                entries,
                feeds: self.metas,
            },
        }
    }
}
