use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::{
    policy::{CrawlPolicy, PollingPolicy},
    target_list::{CrawlTarget, CrawlTargetState},
};

/// Pure schedule synchronization decision for one reconciler tick.
#[derive(Debug, Clone)]
pub struct ScheduleSync {
    now: DateTime<Utc>,
}

impl ScheduleSync {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }

    pub fn upsert_command(&self, entry: &ScheduleSyncEntry) -> Option<UpsertCrawlScheduleCommand> {
        let next_crawl_after = self.next_crawl_after(entry);
        let needs_upsert = entry.schedule.as_ref().is_none_or(|schedule| {
            schedule.target_updated_at != entry.target.target_updated_at
                || schedule.next_crawl_after != next_crawl_after
        });

        needs_upsert.then(|| {
            UpsertCrawlScheduleCommand::new(
                entry.target.feed_url.clone(),
                entry.target.target_updated_at,
                next_crawl_after,
                self.now,
            )
        })
    }

    pub fn crawl_finished_command(
        &self,
        entry: &ScheduleSyncEntry,
        finished_at: DateTime<Utc>,
    ) -> Option<UpsertCrawlScheduleCommand> {
        let next_crawl_after = Self::next_crawl_after_finished(entry, finished_at);
        let needs_upsert = entry.schedule.as_ref().is_none_or(|schedule| {
            schedule.target_updated_at != entry.target.target_updated_at
                || schedule.next_crawl_after != next_crawl_after
        });

        needs_upsert.then(|| {
            UpsertCrawlScheduleCommand::new(
                entry.target.feed_url.clone(),
                entry.target.target_updated_at,
                next_crawl_after,
                self.now,
            )
        })
    }

    fn next_crawl_after(&self, entry: &ScheduleSyncEntry) -> Option<DateTime<Utc>> {
        let ScheduledCrawlTargetState::Active { policy } = entry.target.state else {
            return None;
        };
        let PollingPolicy::Interval { interval } = policy.polling else {
            return None;
        };

        let current_interval_next = PollingPolicy::interval(interval)
            .next_after(self.now)
            .expect("interval policy computes a next crawl time");
        let Some(schedule) = &entry.schedule else {
            return Some(self.now);
        };

        if schedule.target_updated_at != entry.target.target_updated_at {
            return Some(match schedule.next_crawl_after {
                None => self.now,
                Some(previous) => previous.min(current_interval_next),
            });
        }

        Some(schedule.next_crawl_after.unwrap_or(self.now))
    }

    fn next_crawl_after_finished(
        entry: &ScheduleSyncEntry,
        finished_at: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        let ScheduledCrawlTargetState::Active { policy } = entry.target.state else {
            return None;
        };
        let PollingPolicy::Interval { interval } = policy.polling else {
            return None;
        };

        PollingPolicy::interval(interval).next_after(finished_at)
    }
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

/// Durable crawl schedule fact whose next crawl time is due.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledDue {
    pub feed_url: FeedUrl,
    pub due_at: DateTime<Utc>,
}

impl ScheduledDue {
    pub fn new(feed_url: FeedUrl, due_at: DateTime<Utc>) -> Self {
        Self { feed_url, due_at }
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
