use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::{
    job::{ActiveCrawlJob, CrawlJobQueueLane, CrawlJobTrigger, EnqueueCrawlJobCommand},
    policy::{CrawlPolicy, PollingPolicy},
    target_list::{CrawlTarget, CrawlTargetState},
};

/// Pure scheduling decision engine for one reconciler tick.
#[derive(Debug, Clone)]
pub struct CrawlSchedulingEngine {
    now: DateTime<Utc>,
}

impl CrawlSchedulingEngine {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }

    pub fn reconcile(&mut self, candidate: &CrawlScheduleCandidate) -> CrawlScheduleReconciliation {
        let target_changed = candidate.schedule.as_ref().is_none_or(|schedule| {
            schedule.target_updated_at != candidate.target.target_updated_at
        });
        let next_crawl_after = self.next_crawl_after(candidate);
        let schedule = self.upsert_schedule(candidate, next_crawl_after);
        let job = self.enqueue_job(candidate, next_crawl_after, target_changed);

        CrawlScheduleReconciliation { schedule, job }
    }

    fn next_crawl_after(&self, candidate: &CrawlScheduleCandidate) -> Option<DateTime<Utc>> {
        let ScheduledCrawlTargetState::Active { policy } = candidate.target.state else {
            return None;
        };
        let PollingPolicy::Interval { interval } = policy.polling else {
            return None;
        };

        let current_interval_next = PollingPolicy::interval(interval)
            .next_after(self.now)
            .expect("interval policy computes a next crawl time");
        let Some(schedule) = &candidate.schedule else {
            return Some(self.now);
        };

        let target_changed_after_schedule =
            schedule.target_updated_at != candidate.target.target_updated_at;

        if target_changed_after_schedule {
            return Some(match schedule.next_crawl_after {
                // No previous automatic due time means automatic scheduling is
                // starting now, for example manual -> interval.
                None => self.now,
                // Do not push an already-soon crawl farther into the future.
                // If the old due time is too far away for the current policy,
                // pull it back to the next time implied by the current interval.
                Some(previous) => previous.min(current_interval_next),
            });
        }

        Some(schedule.next_crawl_after.unwrap_or(self.now))
    }

    fn upsert_schedule(
        &self,
        candidate: &CrawlScheduleCandidate,
        next_crawl_after: Option<DateTime<Utc>>,
    ) -> Option<UpsertCrawlScheduleCommand> {
        let needs_upsert = candidate.schedule.as_ref().is_none_or(|schedule| {
            schedule.target_updated_at != candidate.target.target_updated_at
                || schedule.next_crawl_after != next_crawl_after
        });

        needs_upsert.then(|| {
            UpsertCrawlScheduleCommand::new(
                candidate.target.feed_url.clone(),
                candidate.target.target_updated_at,
                next_crawl_after,
                self.now,
            )
        })
    }

    fn enqueue_job(
        &self,
        candidate: &CrawlScheduleCandidate,
        next_crawl_after: Option<DateTime<Utc>>,
        target_changed: bool,
    ) -> Option<EnqueueCrawlJobCommand> {
        let due_at = next_crawl_after?;
        let run_after = candidate.readiness.run_after(due_at);

        if run_after > self.now || candidate.active_job.is_some() {
            return None;
        }

        Some(EnqueueCrawlJobCommand::new(
            candidate.target.feed_url.clone(),
            if target_changed {
                CrawlJobTrigger::TargetChanged
            } else {
                CrawlJobTrigger::PeriodicDue
            },
            CrawlJobQueueLane::Default,
            0,
            run_after,
            self.now,
        ))
    }
}

/// One feed's scheduling facts read from durable registry state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlScheduleCandidate {
    pub target: ScheduledCrawlTarget,
    pub schedule: Option<CrawlSchedule>,
    pub active_job: Option<ActiveCrawlJob>,
    pub readiness: CrawlReadiness,
}

impl CrawlScheduleCandidate {
    pub fn new(
        target: ScheduledCrawlTarget,
        schedule: Option<CrawlSchedule>,
        active_job: Option<ActiveCrawlJob>,
        readiness: CrawlReadiness,
    ) -> Self {
        Self {
            target,
            schedule,
            active_job,
            readiness,
        }
    }
}

/// Row read from durable target/schedule state to synchronize `crawl_schedule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleSyncRow {
    pub target: ScheduledCrawlTarget,
    pub schedule: Option<CrawlSchedule>,
}

impl ScheduleSyncRow {
    pub fn new(target: ScheduledCrawlTarget, schedule: Option<CrawlSchedule>) -> Self {
        Self { target, schedule }
    }
}

/// `crawl_target` facts needed by the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledCrawlTarget {
    pub feed_url: FeedUrl,
    pub target_updated_at: DateTime<Utc>,
    pub state: ScheduledCrawlTargetState,
}

impl ScheduledCrawlTarget {
    pub fn new(
        feed_url: FeedUrl,
        target_updated_at: DateTime<Utc>,
        state: ScheduledCrawlTargetState,
    ) -> Self {
        Self {
            feed_url,
            target_updated_at,
            state,
        }
    }
}

impl From<&CrawlTarget> for ScheduledCrawlTarget {
    fn from(target: &CrawlTarget) -> Self {
        Self::new(
            target.feed_url.clone(),
            target.updated_at,
            (&target.state).into(),
        )
    }
}

/// Scheduler-facing crawl target state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledCrawlTargetState {
    Inactive,
    Active { policy: CrawlPolicy },
}

impl From<&CrawlTargetState> for ScheduledCrawlTargetState {
    fn from(state: &CrawlTargetState) -> Self {
        match state {
            CrawlTargetState::Active {
                effective_policy, ..
            } => Self::Active {
                policy: *effective_policy,
            },
            CrawlTargetState::Inactive => Self::Inactive,
        }
    }
}

/// Durable `crawl_schedule` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlSchedule {
    pub feed_url: FeedUrl,
    pub target_updated_at: DateTime<Utc>,
    pub next_crawl_after: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CrawlSchedule {
    pub fn new(
        feed_url: FeedUrl,
        target_updated_at: DateTime<Utc>,
        next_crawl_after: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            target_updated_at,
            next_crawl_after,
            created_at,
            updated_at,
        }
    }
}

/// Crawl observation/backpressure readiness facts for one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlReadiness {
    pub not_before: Option<DateTime<Utc>>,
}

impl CrawlReadiness {
    pub fn ready() -> Self {
        Self { not_before: None }
    }

    pub fn run_after(&self, due_at: DateTime<Utc>) -> DateTime<Utc> {
        self.not_before
            .map_or(due_at, |not_before| due_at.max(not_before))
    }
}

/// Result of reconciling one scheduling candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlScheduleReconciliation {
    pub schedule: Option<UpsertCrawlScheduleCommand>,
    pub job: Option<EnqueueCrawlJobCommand>,
}

/// Command to create or update one schedule row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertCrawlScheduleCommand {
    pub feed_url: FeedUrl,
    pub target_updated_at: DateTime<Utc>,
    pub next_crawl_after: Option<DateTime<Utc>>,
    pub reconciled_at: DateTime<Utc>,
}

impl UpsertCrawlScheduleCommand {
    pub fn new(
        feed_url: FeedUrl,
        target_updated_at: DateTime<Utc>,
        next_crawl_after: Option<DateTime<Utc>>,
        reconciled_at: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            target_updated_at,
            next_crawl_after,
            reconciled_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::crawl::{
        job::{CrawlJobId, CrawlJobState},
        policy::PollingInterval,
    };

    fn interval(seconds: u64) -> CrawlPolicy {
        CrawlPolicy::interval(PollingInterval::try_from(Duration::from_secs(seconds)).unwrap())
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn target(state: ScheduledCrawlTargetState, updated_at: DateTime<Utc>) -> ScheduledCrawlTarget {
        ScheduledCrawlTarget::new(feed_url(), updated_at, state)
    }

    fn candidate(
        state: ScheduledCrawlTargetState,
        target_updated_at: DateTime<Utc>,
        schedule: Option<CrawlSchedule>,
    ) -> CrawlScheduleCandidate {
        CrawlScheduleCandidate::new(
            target(state, target_updated_at),
            schedule,
            None,
            CrawlReadiness::ready(),
        )
    }

    fn schedule(
        target_updated_at: DateTime<Utc>,
        next_crawl_after: Option<DateTime<Utc>>,
    ) -> CrawlSchedule {
        CrawlSchedule::new(
            feed_url(),
            target_updated_at,
            next_crawl_after,
            now(),
            now(),
        )
    }

    fn engine() -> CrawlSchedulingEngine {
        CrawlSchedulingEngine::new(now())
    }

    #[test]
    fn creates_due_schedule_and_target_changed_job_for_new_interval_target() {
        let mut engine = engine();
        let candidate = candidate(
            ScheduledCrawlTargetState::Active {
                policy: interval(3600),
            },
            now(),
            None,
        );
        let reconciliation = engine.reconcile(&candidate);

        assert_eq!(
            reconciliation.schedule,
            Some(UpsertCrawlScheduleCommand::new(
                feed_url(),
                now(),
                Some(now()),
                now()
            ))
        );
        assert_eq!(
            reconciliation.job.map(|job| job.trigger),
            Some(CrawlJobTrigger::TargetChanged)
        );
    }

    #[test]
    fn creates_null_schedule_for_manual_target_without_job() {
        let mut engine = engine();
        let candidate = candidate(
            ScheduledCrawlTargetState::Active {
                policy: CrawlPolicy::manual(),
            },
            now(),
            None,
        );
        let reconciliation = engine.reconcile(&candidate);

        assert_eq!(
            reconciliation.schedule,
            Some(UpsertCrawlScheduleCommand::new(
                feed_url(),
                now(),
                None,
                now()
            ))
        );
        assert_eq!(reconciliation.job, None);
    }

    #[test]
    fn due_existing_schedule_enqueues_periodic_job_without_updating_schedule() {
        let mut engine = engine();
        let target_updated_at = now() - chrono::Duration::hours(2);
        let due_at = now() - chrono::Duration::minutes(1);
        let candidate = candidate(
            ScheduledCrawlTargetState::Active {
                policy: interval(3600),
            },
            target_updated_at,
            Some(schedule(target_updated_at, Some(due_at))),
        );
        let reconciliation = engine.reconcile(&candidate);

        assert_eq!(reconciliation.schedule, None);
        assert_eq!(
            reconciliation.job.map(|job| (job.trigger, job.run_after)),
            Some((CrawlJobTrigger::PeriodicDue, due_at))
        );
    }

    #[test]
    fn active_job_suppresses_enqueue() {
        let mut engine = engine();
        let target_updated_at = now() - chrono::Duration::hours(2);
        let due_at = now() - chrono::Duration::minutes(1);
        let mut candidate = candidate(
            ScheduledCrawlTargetState::Active {
                policy: interval(3600),
            },
            target_updated_at,
            Some(schedule(target_updated_at, Some(due_at))),
        );
        candidate.active_job = Some(ActiveCrawlJob::new(
            CrawlJobId::new("job"),
            CrawlJobState::Pending,
        ));

        let reconciliation = engine.reconcile(&candidate);

        assert_eq!(reconciliation.schedule, None);
        assert_eq!(reconciliation.job, None);
    }

    #[test]
    fn stale_interval_schedule_moves_far_future_due_time_inside_current_interval() {
        let mut engine = engine();
        let old_target_updated_at = now() - chrono::Duration::days(1);
        let new_target_updated_at = now();
        let far_future = now() + chrono::Duration::days(365);
        let candidate = candidate(
            ScheduledCrawlTargetState::Active {
                policy: interval(86_400),
            },
            new_target_updated_at,
            Some(schedule(old_target_updated_at, Some(far_future))),
        );
        let reconciliation = engine.reconcile(&candidate);

        assert_eq!(
            reconciliation.schedule,
            Some(UpsertCrawlScheduleCommand::new(
                feed_url(),
                new_target_updated_at,
                Some(now() + chrono::Duration::days(1)),
                now()
            ))
        );
        assert_eq!(reconciliation.job, None);
    }

    #[test]
    fn stale_interval_schedule_from_null_starts_now() {
        let mut engine = engine();
        let old_target_updated_at = now() - chrono::Duration::days(1);
        let new_target_updated_at = now();
        let candidate = candidate(
            ScheduledCrawlTargetState::Active {
                policy: interval(3600),
            },
            new_target_updated_at,
            Some(schedule(old_target_updated_at, None)),
        );
        let reconciliation = engine.reconcile(&candidate);

        assert_eq!(
            reconciliation.schedule,
            Some(UpsertCrawlScheduleCommand::new(
                feed_url(),
                new_target_updated_at,
                Some(now()),
                now()
            ))
        );
        assert_eq!(
            reconciliation.job.map(|job| job.trigger),
            Some(CrawlJobTrigger::TargetChanged)
        );
    }
}
