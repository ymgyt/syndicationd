use std::{fmt, time::Duration};

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::{job::CrawlJobTrigger, policy::PollingPolicy, state::CrawlState};

/// Base delay applied to the first crawl retry after a failure.
const RETRY_BACKOFF_BASE: Duration = Duration::from_mins(1);

/// Exponent cap keeping the retry backoff below `60s * 2^8` (~4.3h).
const RETRY_BACKOFF_MAX_EXPONENT: u32 = 8;

/// Why a feed's next crawl is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueReason {
    Periodic,
    Manual,
    Retry,
}

impl DueReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::Manual => "manual",
            Self::Retry => "retry",
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

/// Durable facts one active crawl target's next crawl is derived from.
///
/// Schedules are not persisted: every scheduler pass re-reads these facts
/// and evaluates them, so restarts and missed wakes recover by themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlDueInput {
    pub feed_url: FeedUrl,
    pub polling: PollingPolicy,
    /// Pending manual crawl request, cleared by the crawl that serves it.
    pub manual_requested_at: Option<DateTime<Utc>>,
    pub state: Option<CrawlState>,
}

/// The scheduler's decision for one feed at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlDueDecision {
    Due(CrawlDue),
    /// Not due yet; the instant it becomes due.
    Wait(DateTime<Utc>),
    /// Nothing schedules this feed (manual-only policy, no pending request).
    Dormant,
}

/// A feed selected as due for crawling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlDue {
    pub feed_url: FeedUrl,
    pub due_at: DateTime<Utc>,
    pub reason: DueReason,
}

impl CrawlDueInput {
    /// Pure due evaluation over durable facts:
    /// a pending manual request is due immediately; otherwise the next
    /// periodic instant follows the last crawl, with failed crawls retried
    /// on a capped exponential backoff that honors `Retry-After`.
    pub fn evaluate(&self, now: DateTime<Utc>) -> CrawlDueDecision {
        if let Some(requested_at) = self.manual_requested_at {
            return CrawlDueDecision::Due(CrawlDue {
                feed_url: self.feed_url.clone(),
                due_at: requested_at,
                reason: DueReason::Manual,
            });
        }

        let PollingPolicy::Interval { interval } = self.polling else {
            return CrawlDueDecision::Dormant;
        };
        let Some(state) = &self.state else {
            // Never crawled: due immediately.
            return CrawlDueDecision::Due(CrawlDue {
                feed_url: self.feed_url.clone(),
                due_at: now,
                reason: DueReason::Periodic,
            });
        };

        let (next, reason) = if state.last.is_normal() {
            (
                add_duration(state.last.finished_at, interval.duration()),
                DueReason::Periodic,
            )
        } else {
            // Retry never waits longer than the regular cadence.
            let delay = retry_backoff(state.health.failure_streak.value()).min(interval.duration());
            let mut next = add_duration(state.last.finished_at, delay);
            if let Some(retry_after) = state.last.retry_after {
                next = next.max(retry_after);
            }
            (next, DueReason::Retry)
        };

        if next <= now {
            CrawlDueDecision::Due(CrawlDue {
                feed_url: self.feed_url.clone(),
                due_at: next,
                reason,
            })
        } else {
            CrawlDueDecision::Wait(next)
        }
    }
}

/// Exponential retry backoff derived from the consecutive failure count.
fn retry_backoff(failure_streak: u64) -> Duration {
    let exponent = failure_streak
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
        state::{CrawlHealth, CrawlStateError, FailureStreak, LastCrawlResult},
    };

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap()
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn interval_policy(duration: Duration) -> PollingPolicy {
        PollingPolicy::interval(PollingInterval::try_from(duration).unwrap())
    }

    fn input(
        polling: PollingPolicy,
        manual_requested_at: Option<DateTime<Utc>>,
        state: Option<CrawlState>,
    ) -> CrawlDueInput {
        CrawlDueInput {
            feed_url: feed_url(),
            polling,
            manual_requested_at,
            state,
        }
    }

    fn normal_state(finished_at: DateTime<Utc>) -> CrawlState {
        CrawlState {
            feed_url: feed_url(),
            last: LastCrawlResult::normal(finished_at, finished_at, None, None),
            health: CrawlHealth::healthy(),
            conditional: FeedConditionalFetch::default(),
        }
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
        }
    }

    #[test]
    fn manual_request_is_due_immediately() {
        let requested_at = now() - chrono::Duration::seconds(5);
        let decision = input(PollingPolicy::manual(), Some(requested_at), None).evaluate(now());

        assert_eq!(
            decision,
            CrawlDueDecision::Due(CrawlDue {
                feed_url: feed_url(),
                due_at: requested_at,
                reason: DueReason::Manual,
            })
        );
    }

    #[test]
    fn manual_policy_without_request_is_dormant() {
        let decision = input(PollingPolicy::manual(), None, None).evaluate(now());

        assert_eq!(decision, CrawlDueDecision::Dormant);
    }

    #[test]
    fn never_crawled_interval_target_is_due_now() {
        let decision = input(interval_policy(Duration::from_hours(1)), None, None).evaluate(now());

        assert_eq!(
            decision,
            CrawlDueDecision::Due(CrawlDue {
                feed_url: feed_url(),
                due_at: now(),
                reason: DueReason::Periodic,
            })
        );
    }

    #[test]
    fn next_periodic_crawl_follows_last_finish() {
        let finished_at = now() - chrono::Duration::minutes(30);
        let decision = input(
            interval_policy(Duration::from_hours(1)),
            None,
            Some(normal_state(finished_at)),
        )
        .evaluate(now());

        assert_eq!(
            decision,
            CrawlDueDecision::Wait(finished_at + chrono::Duration::hours(1))
        );
    }

    #[test]
    fn elapsed_interval_is_due_at_its_instant() {
        let finished_at = now() - chrono::Duration::hours(2);
        let decision = input(
            interval_policy(Duration::from_hours(1)),
            None,
            Some(normal_state(finished_at)),
        )
        .evaluate(now());

        assert_eq!(
            decision,
            CrawlDueDecision::Due(CrawlDue {
                feed_url: feed_url(),
                due_at: finished_at + chrono::Duration::hours(1),
                reason: DueReason::Periodic,
            })
        );
    }

    #[test]
    fn failed_crawl_retries_with_backoff() {
        // streak 3 -> 60s * 2^2 = 240s
        let decision = input(
            interval_policy(Duration::from_hours(1)),
            None,
            Some(failed_state(3, None)),
        )
        .evaluate(now() + chrono::Duration::seconds(300));

        assert_eq!(
            decision,
            CrawlDueDecision::Due(CrawlDue {
                feed_url: feed_url(),
                due_at: now() + chrono::Duration::seconds(240),
                reason: DueReason::Retry,
            })
        );
    }

    #[test]
    fn retry_backoff_is_capped_by_polling_interval() {
        let decision = input(
            interval_policy(Duration::from_mins(2)),
            None,
            Some(failed_state(10, None)),
        )
        .evaluate(now() + chrono::Duration::minutes(3));

        assert_eq!(
            decision,
            CrawlDueDecision::Due(CrawlDue {
                feed_url: feed_url(),
                due_at: now() + chrono::Duration::minutes(2),
                reason: DueReason::Retry,
            })
        );
    }

    #[test]
    fn retry_honors_retry_after() {
        let retry_after = now() + chrono::Duration::hours(3);
        let decision = input(
            interval_policy(Duration::from_hours(1)),
            None,
            Some(failed_state(1, Some(retry_after))),
        )
        .evaluate(now());

        assert_eq!(decision, CrawlDueDecision::Wait(retry_after));
    }

    #[test]
    fn retry_backoff_growth_is_capped() {
        assert_eq!(retry_backoff(0), RETRY_BACKOFF_BASE);
        assert_eq!(retry_backoff(1), RETRY_BACKOFF_BASE);
        assert_eq!(retry_backoff(2), RETRY_BACKOFF_BASE * 2);
        assert_eq!(
            retry_backoff(100),
            RETRY_BACKOFF_BASE * 2u32.pow(RETRY_BACKOFF_MAX_EXPONENT)
        );
    }
}
