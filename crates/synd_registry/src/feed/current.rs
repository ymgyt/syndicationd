use bon::Builder;
use chrono::{DateTime, Utc};
use synd_feed::types::{FeedMeta, FeedUrl};

use crate::{
    crawl::{blob::BlobRef, job::CrawlJobId, result::CrawlResultRef},
    db::{BlobStoreTx, FeedProjectionTx},
    error::RegistryDbResult,
    event::{Event, FeedChangedEvent, FeedDiscoveredEvent},
};

/// Source crawl result used to derive the current feed state.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct FeedSource {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
    pub result_ref: CrawlResultRef,
    pub body_blob: BlobRef,
    pub seen_at: DateTime<Utc>,
}

/// Command to replace the current feed state with the latest parsed result.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct UpsertFeedCommand {
    pub source: FeedSource,
    pub meta: FeedMeta,
}

/// Result of applying one parsed feed to the current feed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertFeedOutcome {
    Discovered,
    Changed,
    Unchanged,
}

impl UpsertFeedOutcome {
    /// Returns the feed lifecycle event represented by this write outcome.
    pub fn into_event(self, source: &FeedSource) -> Option<Event> {
        match self {
            Self::Discovered => Some(
                FeedDiscoveredEvent::new(source.feed_url.clone(), source.crawl_job_id.clone())
                    .into(),
            ),
            Self::Changed => Some(
                FeedChangedEvent::new(source.feed_url.clone(), source.crawl_job_id.clone()).into(),
            ),
            Self::Unchanged => None,
        }
    }
}

/// Transaction-scoped operations for projecting crawled feed state.
pub struct FeedProjectionScope<'a, Tx> {
    tx: &'a mut Tx,
}

impl<'a, Tx> FeedProjectionScope<'a, Tx> {
    /// Creates a projection scope inside one open registry transaction.
    pub fn new(tx: &'a mut Tx) -> Self {
        Self { tx }
    }
}

impl<Tx> FeedProjectionScope<'_, Tx>
where
    Tx: BlobStoreTx + FeedProjectionTx + Send,
{
    /// Returns the crawl result source that can update feed state.
    pub async fn load_feed_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        self.tx.load_feed_source(job_id).await
    }

    /// Returns the fetched response body for the given feed source.
    pub async fn load_body(&mut self, source: &FeedSource) -> RegistryDbResult<Vec<u8>> {
        self.tx.load_blob(source.body_blob).await
    }

    /// Applies the latest parsed feed state and returns feed lifecycle events.
    pub async fn upsert_feed(
        &mut self,
        command: UpsertFeedCommand,
    ) -> RegistryDbResult<(UpsertFeedOutcome, Vec<Event>)> {
        let source = command.source.clone();
        let outcome = self.tx.upsert_feed(command).await?;
        let events = outcome.into_event(&source).into_iter().collect();
        Ok((outcome, events))
    }
}
