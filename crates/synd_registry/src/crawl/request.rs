use std::sync::Arc;

use synd_feed::types::FeedUrl;
use synd_support::time::Clock;
use thiserror::Error;
use tracing::info;

use crate::{
    command::{RequestCrawlCommand, RequestCrawlOutput},
    crawl::due::CrawlDueInput,
    db::{CommitTx, CrawlTargetStore, FeedRegistryDb},
    error::FeedRegistryError,
    event::{CrawlRequestedEvent, EventJournalAppend, EventRecorder, RecordedEvents},
    handler::{CommandHandler, HandledCommand},
};

/// Result of applying a crawl request to current crawl-target state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestCrawlOutcome {
    /// The request was recorded; the scheduler dispatches it immediately.
    Requested,
    /// An earlier manual request is already waiting to be served.
    AlreadyPending,
}

impl RequestCrawlOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::AlreadyPending => "already_pending",
        }
    }
}

/// Domain rejection returned before any state mutation or journal append.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CrawlRequestReject {
    #[error("feed is not an active crawl target: {0}")]
    NotActiveTarget(FeedUrl),
}

/// Decision made for one crawl request: the caller-visible outcome and
/// whether the pending-request fact must be written and recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CrawlRequestDecision {
    outcome: RequestCrawlOutcome,
    event: Option<CrawlRequestedEvent>,
}

impl CrawlRequestDecision {
    /// Pure decision over current crawl-target state for one crawl request.
    fn decide(
        feed_url: &FeedUrl,
        input: Option<&CrawlDueInput>,
    ) -> Result<Self, CrawlRequestReject> {
        let Some(input) = input else {
            return Err(CrawlRequestReject::NotActiveTarget(feed_url.clone()));
        };
        if input.manual_requested_at.is_some() {
            return Ok(Self {
                outcome: RequestCrawlOutcome::AlreadyPending,
                event: None,
            });
        }
        Ok(Self {
            outcome: RequestCrawlOutcome::Requested,
            event: Some(CrawlRequestedEvent::new(feed_url.clone())),
        })
    }
}

/// Handles crawl requests as a pending-request fact on the crawl target.
///
/// The request is served by the scheduler: a pending `manual_requested_at`
/// makes the feed due immediately, and the crawl that serves it clears the
/// fact on completion.
#[derive(Clone)]
pub(crate) struct CrawlRequestHandler<S> {
    db: S,
    clock: Arc<dyn Clock>,
}

impl<S> CrawlRequestHandler<S> {
    pub(crate) fn new(db: S, clock: Arc<dyn Clock>) -> Self {
        Self { db, clock }
    }
}

impl<S> CommandHandler<RequestCrawlCommand> for CrawlRequestHandler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlTargetStore + EventJournalAppend,
{
    type Output = RequestCrawlOutput;
    type Error = FeedRegistryError;

    async fn handle(
        &self,
        command: RequestCrawlCommand,
    ) -> Result<HandledCommand<Self::Output>, Self::Error> {
        let feed_url = command.feed_url;
        let now = self.clock.now();

        let mut tx = self.db.begin().await?;
        let input = tx.load_crawl_due_input(&feed_url).await?;
        let decision = CrawlRequestDecision::decide(&feed_url, input.as_ref())?;

        let mut recorded_events = RecordedEvents::with_capacity(1);
        if decision.event.is_some() {
            tx.set_manual_request(&feed_url, now).await?;
        }
        EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref())
            .record_all(decision.event)
            .await?;
        tx.commit().await?;

        info!(
            feed_url = feed_url.as_str(),
            outcome = decision.outcome.as_str(),
            "crawl request committed"
        );

        Ok(HandledCommand {
            output: RequestCrawlOutput {
                outcome: decision.outcome,
            },
            recorded_events,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeZone, Utc};

    use super::*;
    use crate::crawl::policy::PollingPolicy;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn input(manual_requested_at: Option<DateTime<Utc>>) -> CrawlDueInput {
        CrawlDueInput {
            feed_url: feed_url(),
            polling: PollingPolicy::manual(),
            manual_requested_at,
            state: None,
        }
    }

    #[test]
    fn rejects_unknown_or_inactive_feed() {
        let result = CrawlRequestDecision::decide(&feed_url(), None);

        assert_eq!(result, Err(CrawlRequestReject::NotActiveTarget(feed_url())));
    }

    #[test]
    fn requests_when_no_manual_request_is_pending() {
        let input = input(None);

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&input));

        assert_eq!(
            result,
            Ok(CrawlRequestDecision {
                outcome: RequestCrawlOutcome::Requested,
                event: Some(CrawlRequestedEvent::new(feed_url())),
            })
        );
    }

    #[test]
    fn reports_already_pending_for_outstanding_request() {
        let input = input(Some(now()));

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&input));

        assert_eq!(
            result,
            Ok(CrawlRequestDecision {
                outcome: RequestCrawlOutcome::AlreadyPending,
                event: None,
            })
        );
    }
}
