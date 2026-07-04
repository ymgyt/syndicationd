use std::{fmt, str::FromStr, time::Duration};

use bon::Builder;
use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::{
    job::CrawlJobTrigger,
    policy::PollingPolicy,
    result::{CrawlState, FailureStreak},
    target_list::{CrawlTarget, CrawlTargetState},
};

mod projection;

pub use projection::{CrawlScheduleProj, CrawlScheduleProjInput};

/// Base delay applied to the first crawl retry after a failure.
const RETRY_BACKOFF_BASE: Duration = Duration::from_mins(1);

/// Exponent cap keeping the retry backoff below `60s * 2^8` (~4.3h).
const RETRY_BACKOFF_MAX_EXPONENT: u32 = 8;

/// Why a schedule row's next crawl is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueReason {
    Periodic,
    Manual,
    Retry,
}

impl DueReason {
    pub const PERIODIC: &'static str = "periodic";
    pub const MANUAL: &'static str = "manual";
    pub const RETRY: &'static str = "retry";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Periodic => Self::PERIODIC,
            Self::Manual => Self::MANUAL,
            Self::Retry => Self::RETRY,
        }
    }

    /// Dispatch precedence: manual requests first, then retries, then periodic dues.
    pub fn dispatch_priority(self) -> u8 {
        match self {
            Self::Manual => 0,
            Self::Retry => 1,
            Self::Periodic => 2,
        }
    }

    pub fn job_trigger(self) -> CrawlJobTrigger {
        match self {
            Self::Periodic => CrawlJobTrigger::PeriodicDue,
            Self::Manual => CrawlJobTrigger::ManualRequest,
            Self::Retry => CrawlJobTrigger::RetryDue,
        }
    }
}

impl fmt::Display for DueReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DueReason {
    type Err = UnknownDueReason;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::PERIODIC => Ok(Self::Periodic),
            Self::MANUAL => Ok(Self::Manual),
            Self::RETRY => Ok(Self::Retry),
            value => Err(UnknownDueReason {
                value: value.to_owned(),
            }),
        }
    }
}

/// Error returned when a stored due reason is unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDueReason {
    value: String,
}

impl fmt::Display for UnknownDueReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown crawl due reason: {}", self.value)
    }
}

impl std::error::Error for UnknownDueReason {}

/// Durable `crawl_schedule` row.
///
/// `dispatched_at` marks an inflight dispatch: the row is not handed to the
/// dispatch queue again until the marker is cleared by the finished crawl or
/// its stale deadline passes (crash recovery).
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct CrawlSchedule {
    pub feed_url: FeedUrl,
    pub target_updated_at: DateTime<Utc>,
    pub next_crawl_after: Option<DateTime<Utc>>,
    pub due_reason: DueReason,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CrawlSchedule {
    /// Whether the dispatcher may hand this row to the dispatch queue: a due
    /// row that is not inflight, or an inflight row whose dispatch went stale.
    ///
    /// The SQL predicate in the sqlite store mirrors this definition.
    pub fn is_dispatchable(&self, now: DateTime<Utc>, stale_before: DateTime<Utc>) -> bool {
        match self.dispatched_at {
            None => self
                .next_crawl_after
                .is_some_and(|next_crawl_after| next_crawl_after <= now),
            Some(dispatched_at) => dispatched_at <= stale_before,
        }
    }

    /// The next future instant this row requires dispatcher attention: its
    /// due time when idle, or its stale deadline when inflight.
    ///
    /// The SQL in the sqlite store mirrors this definition.
    pub fn next_dispatch_wake(
        &self,
        now: DateTime<Utc>,
        stale_timeout: Duration,
    ) -> Option<DateTime<Utc>> {
        match self.dispatched_at {
            None => self
                .next_crawl_after
                .filter(|next_crawl_after| *next_crawl_after > now),
            Some(dispatched_at) => Some(add_duration(dispatched_at, stale_timeout)),
        }
    }
}

/// Command to create or update one schedule row, preserving any inflight
/// dispatch marker.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct UpsertCrawlScheduleCommand {
    pub feed_url: FeedUrl,
    pub target_updated_at: DateTime<Utc>,
    pub next_crawl_after: Option<DateTime<Utc>>,
    pub due_reason: DueReason,
    pub synced_at: DateTime<Utc>,
}

/// Command applied when a dispatched crawl finished: clears the inflight
/// dispatch marker and schedules the next crawl.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct CompleteDispatchCommand {
    pub feed_url: FeedUrl,
    pub target_updated_at: DateTime<Utc>,
    pub next_crawl_after: Option<DateTime<Utc>>,
    pub due_reason: DueReason,
    pub synced_at: DateTime<Utc>,
}

/// A schedule row selected for dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchCandidate {
    pub feed_url: FeedUrl,
    pub due_at: DateTime<Utc>,
    pub due_reason: DueReason,
}

/// Entry read from durable target/schedule state to synchronize `crawl_schedule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleSyncEntry {
    pub target: ScheduledCrawlTarget,
    pub schedule: Option<CrawlSchedule>,
}

impl ScheduleSyncEntry {
    pub fn new(target: ScheduledCrawlTarget, schedule: Option<CrawlSchedule>) -> Self {
        Self { target, schedule }
    }
}

/// `crawl_target` facts needed by the schedule projection.
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

/// Schedule-facing crawl target state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledCrawlTargetState {
    Inactive,
    Active { polling: PollingPolicy },
}

impl From<&CrawlTargetState> for ScheduledCrawlTargetState {
    fn from(state: &CrawlTargetState) -> Self {
        match state {
            CrawlTargetState::Active {
                effective_policy, ..
            } => Self::Active {
                polling: effective_policy.polling,
            },
            CrawlTargetState::Inactive => Self::Inactive,
        }
    }
}

/// Pure schedule synchronization decisions for the schedule projection.
#[derive(Debug, Clone)]
pub struct ScheduleSync {
    now: DateTime<Utc>,
}

impl ScheduleSync {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }

    /// Decision for a target-driven sync (activation, policy change,
    /// deactivation). Pending earlier due times keep their reason.
    pub fn upsert_command(&self, entry: &ScheduleSyncEntry) -> Option<UpsertCrawlScheduleCommand> {
        let (next_crawl_after, due_reason) = self.next_after_target_update(entry);
        let needs_upsert = entry.schedule.as_ref().is_none_or(|schedule| {
            schedule.target_updated_at != entry.target.target_updated_at
                || schedule.next_crawl_after != next_crawl_after
                || schedule.due_reason != due_reason
        });

        needs_upsert.then(|| {
            UpsertCrawlScheduleCommand::builder()
                .feed_url(entry.target.feed_url.clone())
                .target_updated_at(entry.target.target_updated_at)
                .maybe_next_crawl_after(next_crawl_after)
                .due_reason(due_reason)
                .synced_at(self.now)
                .build()
        })
    }

    /// Decision applied when a manual crawl was requested: the schedule
    /// becomes due at the request time unless an equally urgent due is
    /// already pending.
    pub fn manual_request_command(
        &self,
        entry: &ScheduleSyncEntry,
        requested_at: DateTime<Utc>,
    ) -> Option<UpsertCrawlScheduleCommand> {
        let ScheduledCrawlTargetState::Active { .. } = entry.target.state else {
            return None;
        };
        if let Some(schedule) = &entry.schedule
            && schedule.due_reason == DueReason::Manual
            && schedule
                .next_crawl_after
                .is_some_and(|next| next <= requested_at)
        {
            return None;
        }

        Some(
            UpsertCrawlScheduleCommand::builder()
                .feed_url(entry.target.feed_url.clone())
                .target_updated_at(entry.target.target_updated_at)
                .next_crawl_after(requested_at)
                .due_reason(DueReason::Manual)
                .synced_at(self.now)
                .build(),
        )
    }

    /// Decision applied when a dispatched crawl finished. Always returns a
    /// command because the inflight dispatch marker must be cleared even when
    /// the next due time is unchanged.
    pub fn crawl_finished_command(
        &self,
        entry: &ScheduleSyncEntry,
        crawl_state: Option<&CrawlState>,
        finished_at: DateTime<Utc>,
    ) -> CompleteDispatchCommand {
        let (next_crawl_after, due_reason) =
            Self::next_after_finished(entry, crawl_state, finished_at);
        CompleteDispatchCommand::builder()
            .feed_url(entry.target.feed_url.clone())
            .target_updated_at(entry.target.target_updated_at)
            .maybe_next_crawl_after(next_crawl_after)
            .due_reason(due_reason)
            .synced_at(self.now)
            .build()
    }

    fn next_after_target_update(
        &self,
        entry: &ScheduleSyncEntry,
    ) -> (Option<DateTime<Utc>>, DueReason) {
        let ScheduledCrawlTargetState::Active { polling } = entry.target.state else {
            return (None, DueReason::Periodic);
        };
        let PollingPolicy::Interval { .. } = polling else {
            // A manual-only target keeps a pending manual request, otherwise
            // it stays dormant until the next request.
            return match &entry.schedule {
                Some(schedule)
                    if schedule.due_reason == DueReason::Manual
                        && schedule.next_crawl_after.is_some() =>
                {
                    (schedule.next_crawl_after, DueReason::Manual)
                }
                _ => (None, DueReason::Periodic),
            };
        };

        let current_interval_next = polling
            .next_after(self.now)
            .expect("interval policy computes a next crawl time");
        let Some(schedule) = &entry.schedule else {
            return (Some(self.now), DueReason::Periodic);
        };

        if schedule.target_updated_at != entry.target.target_updated_at {
            return match schedule.next_crawl_after {
                None => (Some(self.now), DueReason::Periodic),
                Some(previous) if previous <= current_interval_next => {
                    (Some(previous), schedule.due_reason)
                }
                Some(_) => (Some(current_interval_next), DueReason::Periodic),
            };
        }

        match schedule.next_crawl_after {
            None => (Some(self.now), DueReason::Periodic),
            Some(previous) => (Some(previous), schedule.due_reason),
        }
    }

    fn next_after_finished(
        entry: &ScheduleSyncEntry,
        crawl_state: Option<&CrawlState>,
        finished_at: DateTime<Utc>,
    ) -> (Option<DateTime<Utc>>, DueReason) {
        let ScheduledCrawlTargetState::Active { polling } = entry.target.state else {
            return (None, DueReason::Periodic);
        };
        let PollingPolicy::Interval { interval } = polling else {
            return (None, DueReason::Periodic);
        };

        if let Some(state) = crawl_state
            && !state.last.is_normal()
        {
            let delay = retry_backoff(state.health.failure_streak).min(interval.duration());
            let mut next = add_duration(finished_at, delay);
            if let Some(retry_after) = state.last.retry_after {
                next = next.max(retry_after);
            }
            return (Some(next), DueReason::Retry);
        }

        let next = polling
            .next_after(finished_at)
            .expect("interval policy computes a next crawl time");
        (Some(next), DueReason::Periodic)
    }
}

/// Exponential retry backoff derived from the consecutive failure count.
///
/// The schedule sync additionally caps the delay at the target's polling
/// interval so a retry never waits longer than the regular cadence.
pub fn retry_backoff(failure_streak: FailureStreak) -> Duration {
    let exponent = failure_streak
        .value()
        .saturating_sub(1)
        .min(u64::from(RETRY_BACKOFF_MAX_EXPONENT));
    RETRY_BACKOFF_BASE * 2u32.pow(u32::try_from(exponent).expect("exponent is capped"))
}

fn add_duration(time: DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
    chrono::Duration::from_std(duration).map_or(time, |duration| time + duration)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::TimeZone;
    use synd_feed::feed::service::{FeedConditionalFetch, FeedParseErrorKind};

    use super::*;
    use crate::crawl::{
        policy::PollingInterval,
        result::{CrawlHealth, CrawlStateError, CrawlStateTimestamps, LastCrawlResult},
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn interval(duration: Duration) -> PollingInterval {
        PollingInterval::try_from(duration).unwrap()
    }

    fn active_target(polling: PollingPolicy) -> ScheduledCrawlTarget {
        ScheduledCrawlTarget::new(
            feed_url(),
            now(),
            ScheduledCrawlTargetState::Active { polling },
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

    fn failed_state(failure_streak: u64, retry_after: Option<DateTime<Utc>>) -> CrawlState {
        CrawlState {
            feed_url: feed_url(),
            last: LastCrawlResult::abnormal(
                now(),
                now(),
                None,
                CrawlStateError::parse(FeedParseErrorKind::InvalidFeed),
                retry_after,
            ),
            health: CrawlHealth {
                failure_streak: FailureStreak::new(failure_streak),
            },
            conditional: FeedConditionalFetch::default(),
            timestamps: CrawlStateTimestamps::new(now(), now()),
        }
    }

    #[test]
    fn new_active_target_is_due_immediately() {
        let sync = ScheduleSync::new(now());
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::interval(interval(Duration::from_hours(1)))),
            None,
        );

        let command = sync
            .upsert_command(&entry)
            .expect("upsert should be needed");

        assert_eq!(command.next_crawl_after, Some(now()));
        assert_eq!(command.due_reason, DueReason::Periodic);
    }

    #[test]
    fn unchanged_schedule_needs_no_upsert() {
        let sync = ScheduleSync::new(now());
        let next = now() + chrono::Duration::minutes(30);
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::interval(interval(Duration::from_hours(1)))),
            Some(schedule(Some(next), DueReason::Periodic, None)),
        );

        assert_eq!(sync.upsert_command(&entry), None);
    }

    #[test]
    fn pending_manual_request_survives_policy_change() {
        let sync = ScheduleSync::new(now());
        let mut target = active_target(PollingPolicy::interval(interval(Duration::from_hours(1))));
        target.target_updated_at = now() + chrono::Duration::seconds(1);
        let entry =
            ScheduleSyncEntry::new(target, Some(schedule(Some(now()), DueReason::Manual, None)));

        let command = sync
            .upsert_command(&entry)
            .expect("upsert should be needed");

        assert_eq!(command.next_crawl_after, Some(now()));
        assert_eq!(command.due_reason, DueReason::Manual);
    }

    #[test]
    fn deactivated_target_clears_next_crawl() {
        let sync = ScheduleSync::new(now());
        let entry = ScheduleSyncEntry::new(
            ScheduledCrawlTarget::new(feed_url(), now(), ScheduledCrawlTargetState::Inactive),
            Some(schedule(Some(now()), DueReason::Periodic, None)),
        );

        let command = sync
            .upsert_command(&entry)
            .expect("upsert should be needed");

        assert_eq!(command.next_crawl_after, None);
    }

    #[test]
    fn successful_crawl_schedules_next_interval() {
        let sync = ScheduleSync::new(now());
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::interval(interval(Duration::from_hours(1)))),
            Some(schedule(Some(now()), DueReason::Periodic, Some(now()))),
        );

        let command = sync.crawl_finished_command(&entry, None, now());

        assert_eq!(
            command.next_crawl_after,
            Some(now() + chrono::Duration::hours(1))
        );
        assert_eq!(command.due_reason, DueReason::Periodic);
    }

    #[test]
    fn failed_crawl_schedules_backoff_retry() {
        let sync = ScheduleSync::new(now());
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::interval(interval(Duration::from_hours(1)))),
            Some(schedule(Some(now()), DueReason::Periodic, Some(now()))),
        );
        let state = failed_state(3, None);

        let command = sync.crawl_finished_command(&entry, Some(&state), now());

        // streak 3 -> 60s * 2^2 = 240s
        assert_eq!(
            command.next_crawl_after,
            Some(now() + chrono::Duration::seconds(240))
        );
        assert_eq!(command.due_reason, DueReason::Retry);
    }

    #[test]
    fn retry_backoff_is_capped_by_polling_interval() {
        let sync = ScheduleSync::new(now());
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::interval(interval(Duration::from_mins(2)))),
            Some(schedule(Some(now()), DueReason::Periodic, Some(now()))),
        );
        let state = failed_state(10, None);

        let command = sync.crawl_finished_command(&entry, Some(&state), now());

        assert_eq!(
            command.next_crawl_after,
            Some(now() + chrono::Duration::minutes(2))
        );
        assert_eq!(command.due_reason, DueReason::Retry);
    }

    #[test]
    fn retry_honors_retry_after() {
        let sync = ScheduleSync::new(now());
        let retry_after = now() + chrono::Duration::hours(3);
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::interval(interval(Duration::from_hours(1)))),
            Some(schedule(Some(now()), DueReason::Periodic, Some(now()))),
        );
        let state = failed_state(1, Some(retry_after));

        let command = sync.crawl_finished_command(&entry, Some(&state), now());

        assert_eq!(command.next_crawl_after, Some(retry_after));
        assert_eq!(command.due_reason, DueReason::Retry);
    }

    #[test]
    fn manual_policy_target_goes_dormant_after_crawl() {
        let sync = ScheduleSync::new(now());
        let entry = ScheduleSyncEntry::new(
            active_target(PollingPolicy::manual()),
            Some(schedule(Some(now()), DueReason::Manual, Some(now()))),
        );

        let command = sync.crawl_finished_command(&entry, None, now());

        assert_eq!(command.next_crawl_after, None);
    }

    #[test]
    fn dispatchable_predicate_covers_due_inflight_and_stale() {
        let stale_before = now() - chrono::Duration::minutes(5);

        // due and idle -> dispatchable
        assert!(
            schedule(Some(now()), DueReason::Periodic, None).is_dispatchable(now(), stale_before)
        );
        // future due -> not yet
        assert!(
            !schedule(
                Some(now() + chrono::Duration::hours(1)),
                DueReason::Periodic,
                None
            )
            .is_dispatchable(now(), stale_before)
        );
        // inflight -> blocked
        assert!(
            !schedule(Some(now()), DueReason::Periodic, Some(now()))
                .is_dispatchable(now(), stale_before)
        );
        // stale inflight -> dispatchable again
        assert!(
            schedule(Some(now()), DueReason::Periodic, Some(stale_before))
                .is_dispatchable(now(), stale_before)
        );
        // dormant -> never
        assert!(!schedule(None, DueReason::Periodic, None).is_dispatchable(now(), stale_before));
    }

    #[test]
    fn next_dispatch_wake_is_due_time_or_stale_deadline() {
        let stale_timeout = Duration::from_mins(5);
        let future_due = now() + chrono::Duration::hours(1);

        assert_eq!(
            schedule(Some(future_due), DueReason::Periodic, None)
                .next_dispatch_wake(now(), stale_timeout),
            Some(future_due)
        );
        // already due -> no future wake needed (dispatch happens now)
        assert_eq!(
            schedule(Some(now()), DueReason::Periodic, None)
                .next_dispatch_wake(now(), stale_timeout),
            None
        );
        // inflight -> stale deadline
        assert_eq!(
            schedule(Some(now()), DueReason::Periodic, Some(now()))
                .next_dispatch_wake(now(), stale_timeout),
            Some(now() + chrono::Duration::minutes(5))
        );
        assert_eq!(
            schedule(None, DueReason::Periodic, None).next_dispatch_wake(now(), stale_timeout),
            None
        );
    }

    #[test]
    fn retry_backoff_growth_is_capped() {
        assert_eq!(retry_backoff(FailureStreak::new(0)), RETRY_BACKOFF_BASE);
        assert_eq!(retry_backoff(FailureStreak::new(1)), RETRY_BACKOFF_BASE);
        assert_eq!(retry_backoff(FailureStreak::new(2)), RETRY_BACKOFF_BASE * 2);
        assert_eq!(
            retry_backoff(FailureStreak::new(100)),
            RETRY_BACKOFF_BASE * 2u32.pow(RETRY_BACKOFF_MAX_EXPONENT)
        );
    }
}
