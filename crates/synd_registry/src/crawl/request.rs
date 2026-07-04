use std::sync::Arc;

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use synd_support::time::Clock;
use thiserror::Error;
use tracing::info;

use crate::{
    command::{RequestCrawlCommand, RequestCrawlOutput},
    crawl::schedule::{DueReason, ScheduleSyncEntry, ScheduledCrawlTargetState},
    db::{CommitTx, CrawlScheduleStore, FeedRegistryDb},
    error::FeedRegistryError,
    event::{CrawlRequestedEvent, EventJournalAppend, EventRecorder, RecordedEvents},
    handler::{CommandHandler, HandledCommand},
};

/// Result of applying a crawl request to current schedule state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestCrawlOutcome {
    /// The crawl was scheduled to run immediately.
    Requested,
    /// An earlier manual request is already waiting for dispatch.
    AlreadyPending,
    /// A crawl for the feed is currently running.
    AlreadyRunning,
}

impl RequestCrawlOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::AlreadyPending => "already_pending",
            Self::AlreadyRunning => "already_running",
        }
    }
}

/// Domain rejection returned before any state mutation or journal append.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CrawlRequestReject {
    #[error("feed is not an active crawl target: {0}")]
    NotActiveTarget(FeedUrl),
}

/// Decision made for one crawl request: the caller-visible outcome and the
/// fact to record, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CrawlRequestDecision {
    outcome: RequestCrawlOutcome,
    event: Option<CrawlRequestedEvent>,
}

impl CrawlRequestDecision {
    /// Pure decision over current target/schedule state for one crawl request.
    fn decide(
        feed_url: &FeedUrl,
        entry: Option<&ScheduleSyncEntry>,
        now: DateTime<Utc>,
    ) -> Result<Self, CrawlRequestReject> {
        let Some(entry) = entry else {
            return Err(CrawlRequestReject::NotActiveTarget(feed_url.clone()));
        };
        let ScheduledCrawlTargetState::Active { .. } = entry.target.state else {
            return Err(CrawlRequestReject::NotActiveTarget(feed_url.clone()));
        };

        match &entry.schedule {
            Some(schedule) if schedule.dispatched_at.is_some() => {
                Ok(Self::noop(RequestCrawlOutcome::AlreadyRunning))
            }
            Some(schedule)
                if schedule.due_reason == DueReason::Manual
                    && schedule.next_crawl_after.is_some_and(|next| next <= now) =>
            {
                Ok(Self::noop(RequestCrawlOutcome::AlreadyPending))
            }
            _ => Ok(Self::requested(feed_url)),
        }
    }

    fn requested(feed_url: &FeedUrl) -> Self {
        Self {
            outcome: RequestCrawlOutcome::Requested,
            event: Some(CrawlRequestedEvent::new(feed_url.clone())),
        }
    }

    fn noop(outcome: RequestCrawlOutcome) -> Self {
        Self {
            outcome,
            event: None,
        }
    }
}

/// Handles crawl requests as a decision plus a journaled fact.
///
/// The handler stays thin on purpose: it validates that the request is
/// meaningful and records `CrawlRequested`. The schedule state change is
/// derived event-driven by the crawl schedule projection, which is the sole
/// writer of `crawl_schedule` rows.
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
    for<'tx> S::Tx<'tx>: CrawlScheduleStore + EventJournalAppend,
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
        let entry = tx.load_schedule_sync_entry(&feed_url).await?;
        let decision = CrawlRequestDecision::decide(&feed_url, entry.as_ref(), now)?;

        let mut recorded_events = RecordedEvents::with_capacity(1);
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
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::crawl::{
        policy::PollingPolicy,
        schedule::{CrawlSchedule, ScheduledCrawlTarget},
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn entry(
        state: ScheduledCrawlTargetState,
        schedule: Option<CrawlSchedule>,
    ) -> ScheduleSyncEntry {
        ScheduleSyncEntry::new(
            ScheduledCrawlTarget::new(feed_url(), now(), state),
            schedule,
        )
    }

    fn schedule(
        next_crawl_after: Option<DateTime<Utc>>,
        due_reason: DueReason,
        dispatched_at: Option<DateTime<Utc>>,
    ) -> CrawlSchedule {
        CrawlSchedule::builder()
            .feed_url(feed_url())
            .target_updated_at(now())
            .maybe_next_crawl_after(next_crawl_after)
            .due_reason(due_reason)
            .maybe_dispatched_at(dispatched_at)
            .created_at(now())
            .updated_at(now())
            .build()
    }

    fn active() -> ScheduledCrawlTargetState {
        ScheduledCrawlTargetState::Active {
            polling: PollingPolicy::manual(),
        }
    }

    #[test]
    fn rejects_unknown_feed() {
        let result = CrawlRequestDecision::decide(&feed_url(), None, now());

        assert_eq!(result, Err(CrawlRequestReject::NotActiveTarget(feed_url())));
    }

    #[test]
    fn rejects_inactive_target() {
        let entry = entry(ScheduledCrawlTargetState::Inactive, None);

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&entry), now());

        assert_eq!(result, Err(CrawlRequestReject::NotActiveTarget(feed_url())));
    }

    #[test]
    fn requests_when_schedule_is_idle() {
        let entry = entry(active(), Some(schedule(None, DueReason::Periodic, None)));

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&entry), now());

        assert_eq!(result, Ok(CrawlRequestDecision::requested(&feed_url())));
    }

    #[test]
    fn requests_when_schedule_row_is_missing() {
        let entry = entry(active(), None);

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&entry), now());

        assert_eq!(result, Ok(CrawlRequestDecision::requested(&feed_url())));
    }

    #[test]
    fn requested_decision_records_the_fact() {
        let decision = CrawlRequestDecision::requested(&feed_url());

        assert_eq!(decision.outcome, RequestCrawlOutcome::Requested);
        assert_eq!(decision.event, Some(CrawlRequestedEvent::new(feed_url())));
    }

    #[test]
    fn reports_already_running_while_dispatched() {
        let entry = entry(
            active(),
            Some(schedule(None, DueReason::Manual, Some(now()))),
        );

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&entry), now());

        assert_eq!(
            result,
            Ok(CrawlRequestDecision::noop(
                RequestCrawlOutcome::AlreadyRunning
            ))
        );
    }

    #[test]
    fn reports_already_pending_for_undispatched_manual_due() {
        let entry = entry(
            active(),
            Some(schedule(Some(now()), DueReason::Manual, None)),
        );

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&entry), now());

        assert_eq!(
            result,
            Ok(CrawlRequestDecision::noop(
                RequestCrawlOutcome::AlreadyPending
            ))
        );
    }

    #[test]
    fn periodic_due_can_still_be_requested_manually() {
        let entry = entry(
            active(),
            Some(schedule(
                Some(now() + chrono::Duration::hours(1)),
                DueReason::Periodic,
                None,
            )),
        );

        let result = CrawlRequestDecision::decide(&feed_url(), Some(&entry), now());

        assert_eq!(result, Ok(CrawlRequestDecision::requested(&feed_url())));
    }
}
