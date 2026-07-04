use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::{debug, warn};

use crate::{
    config::CrawlDispatchConfig,
    crawl::{
        dispatch::{DispatchEntry, DispatchQueueWriter},
        schedule::DispatchCandidate,
    },
    db::{CommitTx, CrawlScheduleStore, FeedRegistryDb},
    event::{
        CrawlScheduleUpdatedEvent, EventInterests, Reaction, Reconciler, RecordedEvents,
        RegistryEvent, WakeRequest, WorkerId, WorkerResult,
    },
};

/// Pure claim decision for one dispatcher pass.
///
/// Callers scan `capacity + 1` candidates so the decision can detect that
/// dispatchable rows remain beyond the claimed batch.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DispatchPlan {
    claim: Vec<DispatchCandidate>,
    /// Dispatchable rows remain beyond the claimed batch (queue saturated or
    /// scan limit hit), so the pass must be retried shortly.
    saturated: bool,
}

impl DispatchPlan {
    fn decide(mut candidates: Vec<DispatchCandidate>, capacity: usize) -> Self {
        let saturated = capacity == 0 || candidates.len() > capacity;
        candidates.truncate(capacity);
        Self {
            claim: candidates,
            saturated,
        }
    }

    fn feed_urls(&self) -> Vec<FeedUrl> {
        self.claim
            .iter()
            .map(|candidate| candidate.feed_url.clone())
            .collect()
    }

    /// Pure wake decision after the pass: retry shortly while saturated,
    /// otherwise sleep until the next dispatch instant.
    fn wake_after(
        &self,
        next_dispatch_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
        saturated_retry_delay: std::time::Duration,
    ) -> WakeRequest {
        if self.saturated {
            return chrono::Duration::from_std(saturated_retry_delay)
                .map_or(WakeRequest::None, |delay| WakeRequest::at(now + delay));
        }
        next_dispatch_at.map_or(WakeRequest::None, WakeRequest::at)
    }
}

/// Level-driven reconciler converging due `crawl_schedule` rows into the
/// dispatch queue.
///
/// Desired state: every row whose `next_crawl_after` has passed is either
/// inflight (`dispatched_at` set) or queued. Observed state is re-read from
/// the schedule table on every pass, so restarts and missed wakes recover
/// without any in-memory bookkeeping; a crash between commit and queue push
/// is recovered by the stale-dispatch deadline.
pub(crate) struct CrawlDispatcher {
    queue: DispatchQueueWriter,
    config: CrawlDispatchConfig,
}

impl CrawlDispatcher {
    pub(crate) fn new(queue: DispatchQueueWriter, config: CrawlDispatchConfig) -> Self {
        Self { queue, config }
    }

    fn stale_before(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        chrono::Duration::from_std(self.config.stale_dispatch_timeout)
            .map_or(now, |timeout| now - timeout)
    }

    fn push_claimed(&self, claim: Vec<DispatchCandidate>, now: DateTime<Utc>) {
        for candidate in claim {
            let entry =
                DispatchEntry::new(candidate.feed_url, candidate.due_reason.job_trigger(), now);
            if let Err(err) = self.queue.push(entry) {
                // The row stays marked as dispatched; the stale-dispatch
                // deadline re-dispatches it if the push never happened.
                warn!(error = ?err, "crawl dispatch queue push failed");
            }
        }
    }
}

impl<S> Reconciler<S> for CrawlDispatcher
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlScheduleStore + Send,
{
    fn id(&self) -> WorkerId {
        WorkerId::CrawlDispatcher
    }

    fn wake_hints(&self) -> EventInterests {
        EventInterests::new(vec![CrawlScheduleUpdatedEvent::TYPE])
    }

    async fn reconcile(&mut self, db: &S, now: DateTime<Utc>) -> WorkerResult<Reaction> {
        // observe
        let capacity = self.queue.remaining_capacity();
        let mut tx = db.begin().await?;
        let candidates = if capacity == 0 {
            Vec::new()
        } else {
            tx.list_dispatchable(now, self.stale_before(now), capacity + 1)
                .await?
        };

        // decide
        let plan = DispatchPlan::decide(candidates, capacity);

        // converge
        if !plan.claim.is_empty() {
            tx.mark_dispatched(&plan.feed_urls(), now).await?;
        }
        let next_dispatch_at = tx
            .next_dispatch_at(now, self.config.stale_dispatch_timeout)
            .await?;
        tx.commit().await?;

        let wake = plan.wake_after(next_dispatch_at, now, self.config.saturated_retry_delay);
        debug!(
            dispatched_count = plan.claim.len(),
            saturated = plan.saturated,
            next_dispatch_at = ?next_dispatch_at,
            "crawl dispatcher converged"
        );
        self.push_claimed(plan.claim, now);

        Ok(Reaction::new(RecordedEvents::empty(), wake))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::TimeZone;

    use super::*;
    use crate::crawl::schedule::DueReason;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn candidate(name: &str) -> DispatchCandidate {
        DispatchCandidate {
            feed_url: synd_feed::types::FeedUrl::parse(&format!("https://example.com/{name}.xml"))
                .unwrap(),
            due_at: now(),
            due_reason: DueReason::Periodic,
        }
    }

    #[test]
    fn plan_claims_all_when_within_capacity() {
        let plan = DispatchPlan::decide(vec![candidate("a"), candidate("b")], 4);

        assert_eq!(plan.claim.len(), 2);
        assert!(!plan.saturated);
    }

    #[test]
    fn plan_truncates_and_reports_saturation_beyond_capacity() {
        let plan = DispatchPlan::decide(vec![candidate("a"), candidate("b"), candidate("c")], 2);

        assert_eq!(plan.feed_urls().len(), 2);
        assert!(plan.saturated);
    }

    #[test]
    fn plan_is_saturated_when_queue_is_full() {
        let plan = DispatchPlan::decide(Vec::new(), 0);

        assert!(plan.claim.is_empty());
        assert!(plan.saturated);
    }

    fn plan(saturated: bool) -> DispatchPlan {
        DispatchPlan {
            claim: Vec::new(),
            saturated,
        }
    }

    #[test]
    fn wake_prefers_short_retry_when_saturated() {
        let wake = plan(true).wake_after(
            Some(now() + chrono::Duration::hours(1)),
            now(),
            Duration::from_secs(1),
        );

        assert_eq!(wake, WakeRequest::at(now() + chrono::Duration::seconds(1)));
    }

    #[test]
    fn wake_follows_next_dispatch_when_converged() {
        let next = now() + chrono::Duration::minutes(10);

        assert_eq!(
            plan(false).wake_after(Some(next), now(), Duration::from_secs(1)),
            WakeRequest::at(next)
        );
        assert_eq!(
            plan(false).wake_after(None, now(), Duration::from_secs(1)),
            WakeRequest::None
        );
    }
}
