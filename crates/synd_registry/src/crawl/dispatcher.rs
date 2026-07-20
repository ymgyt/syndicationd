use chrono::{DateTime, Utc};
use tracing::{debug, warn};

use crate::{
    config::CrawlDispatchConfig,
    crawl::{
        dispatch::{DispatchEntry, DispatchQueueWriter, InflightCrawls},
        due::{CrawlDue, CrawlDueDecision},
    },
    db::{CommitTx, CrawlTargetDb, FeedRegistryDb},
    event::{
        CrawlJobFinishedEvent, CrawlRequestedEvent, CrawlTargetActivatedEvent,
        CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, EventInterests, Reaction,
        Reconciler, RecordedEvents, RegistryEvent, WakeRequest, WorkerId, WorkerResult,
    },
};

/// Pure dispatch decision for one scheduler pass over evaluated due inputs.
#[derive(Debug)]
struct DispatchPlan {
    /// Dues to hand to the queue, ordered by dispatch priority and due time.
    claim: Vec<CrawlDue>,
    /// Due feeds remain beyond the claimed batch (queue saturated), so the
    /// pass must be retried shortly.
    saturated: bool,
    /// Earliest future instant a waiting feed becomes due.
    next_due_at: Option<DateTime<Utc>>,
}

impl DispatchPlan {
    fn decide(decisions: Vec<CrawlDueDecision>, capacity: usize) -> Self {
        let mut dues = Vec::new();
        let mut next_due_at: Option<DateTime<Utc>> = None;
        for decision in decisions {
            match decision {
                CrawlDueDecision::Due(due) => dues.push(due),
                CrawlDueDecision::Wait(at) => {
                    next_due_at = Some(next_due_at.map_or(at, |next| next.min(at)));
                }
                CrawlDueDecision::Dormant => {}
            }
        }
        dues.sort_by(|a, b| {
            (a.reason.dispatch_priority(), a.due_at).cmp(&(b.reason.dispatch_priority(), b.due_at))
        });

        let saturated = dues.len() > capacity;
        dues.truncate(capacity);
        Self {
            claim: dues,
            saturated,
            next_due_at,
        }
    }

    /// Pure wake decision after the pass: retry shortly while saturated,
    /// otherwise sleep until the next feed becomes due.
    fn wake_after(
        &self,
        now: DateTime<Utc>,
        saturated_retry_delay: std::time::Duration,
    ) -> WakeRequest {
        if self.saturated {
            return chrono::Duration::from_std(saturated_retry_delay)
                .map_or(WakeRequest::None, |delay| WakeRequest::at(now + delay));
        }
        self.next_due_at.map_or(WakeRequest::None, WakeRequest::at)
    }
}

/// Level-driven scheduler converging due feeds into the dispatch queue.
///
/// Every pass re-reads the durable facts (`crawl_target` + `crawl_state`),
/// derives each feed's next crawl, and hands due feeds to the queue. Nothing
/// about the schedule is persisted, so restarts and missed wakes recover by
/// re-derivation; the in-process inflight set is the only dispatch memory.
pub(crate) struct CrawlDispatcher {
    queue: DispatchQueueWriter,
    inflight: InflightCrawls,
    config: CrawlDispatchConfig,
}

impl CrawlDispatcher {
    pub(crate) fn new(
        queue: DispatchQueueWriter,
        inflight: InflightCrawls,
        config: CrawlDispatchConfig,
    ) -> Self {
        Self {
            queue,
            inflight,
            config,
        }
    }

    fn push_claimed(&self, claim: Vec<CrawlDue>, now: DateTime<Utc>) {
        for due in claim {
            let Some(guard) = self.inflight.try_claim(&due.feed_url) else {
                continue;
            };
            let entry = DispatchEntry::new(due.feed_url, due.reason.job_trigger(), now, guard);
            if let Err(err) = self.queue.push(entry) {
                // The entry drop released the inflight claim; the feed stays
                // due and the next pass re-dispatches it.
                warn!(error = ?err, "crawl dispatch queue push failed");
            }
        }
    }
}

impl<S> Reconciler<S> for CrawlDispatcher
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlTargetDb + Send,
{
    fn id(&self) -> WorkerId {
        WorkerId::CrawlDispatcher
    }

    fn wake_hints(&self) -> EventInterests {
        EventInterests::new(vec![
            CrawlTargetActivatedEvent::TYPE,
            CrawlTargetPolicyChangedEvent::TYPE,
            CrawlTargetDeactivatedEvent::TYPE,
            CrawlRequestedEvent::TYPE,
            CrawlJobFinishedEvent::TYPE,
        ])
    }

    async fn reconcile(&mut self, db: &S, now: DateTime<Utc>) -> WorkerResult<Reaction> {
        // observe
        let capacity = self.queue.remaining_capacity();
        let mut tx = db.begin().await?;
        let inputs = tx.list_crawl_due_inputs().await?;
        tx.commit().await?;

        // decide: inflight feeds are excluded entirely; their completion
        // wake triggers the pass that re-evaluates them
        let decisions = inputs
            .iter()
            .filter(|input| !self.inflight.contains(&input.feed_url))
            .map(|input| input.evaluate(now))
            .collect::<Vec<_>>();
        let plan = DispatchPlan::decide(decisions, capacity);
        let wake = plan.wake_after(now, self.config.saturated_retry_delay);

        // converge
        debug!(
            dispatched_count = plan.claim.len(),
            saturated = plan.saturated,
            next_due_at = ?plan.next_due_at,
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
    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::crawl::due::DueReason;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn due(name: &str, reason: DueReason, due_at: DateTime<Utc>) -> CrawlDueDecision {
        CrawlDueDecision::Due(CrawlDue {
            feed_url: FeedUrl::parse(&format!("https://example.com/{name}.xml")).unwrap(),
            due_at,
            reason,
        })
    }

    #[test]
    fn plan_orders_by_priority_then_due_time() {
        let plan = DispatchPlan::decide(
            vec![
                due("periodic", DueReason::Periodic, now()),
                due("manual", DueReason::Manual, now()),
                due(
                    "retry-late",
                    DueReason::Retry,
                    now() - chrono::Duration::seconds(1),
                ),
                due(
                    "retry-early",
                    DueReason::Retry,
                    now() - chrono::Duration::seconds(10),
                ),
            ],
            4,
        );

        let names = plan
            .claim
            .iter()
            .map(|due| due.feed_url.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "https://example.com/manual.xml",
                "https://example.com/retry-early.xml",
                "https://example.com/retry-late.xml",
                "https://example.com/periodic.xml",
            ]
        );
        assert!(!plan.saturated);
    }

    #[test]
    fn plan_truncates_and_reports_saturation_beyond_capacity() {
        let plan = DispatchPlan::decide(
            vec![
                due("a", DueReason::Periodic, now()),
                due("b", DueReason::Periodic, now()),
                due("c", DueReason::Periodic, now()),
            ],
            2,
        );

        assert_eq!(plan.claim.len(), 2);
        assert!(plan.saturated);
    }

    #[test]
    fn plan_tracks_earliest_wait_instant() {
        let early = now() + chrono::Duration::minutes(10);
        let late = now() + chrono::Duration::hours(1);
        let plan = DispatchPlan::decide(
            vec![
                CrawlDueDecision::Wait(late),
                CrawlDueDecision::Wait(early),
                CrawlDueDecision::Dormant,
            ],
            4,
        );

        assert!(plan.claim.is_empty());
        assert_eq!(plan.next_due_at, Some(early));
        assert_eq!(
            plan.wake_after(now(), Duration::from_secs(1)),
            WakeRequest::at(early)
        );
    }

    #[test]
    fn wake_prefers_short_retry_when_saturated() {
        let plan = DispatchPlan::decide(vec![due("a", DueReason::Periodic, now())], 0);

        assert!(plan.saturated);
        assert_eq!(
            plan.wake_after(now(), Duration::from_secs(1)),
            WakeRequest::at(now() + chrono::Duration::seconds(1))
        );
    }
}
