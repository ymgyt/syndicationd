use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::crawl::{
    job::CrawlJobTrigger,
    scheduler::{
        dispatch::{DispatchBatch, DispatchContext, DispatchEntry},
        input::{ManualRequested, RetryDue, SchedInput, ScheduledDue},
        policy::Scheduler,
    },
};

/// Scheduler-internal ready tier where a crawl candidate is resident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    Manual,
    Scheduled,
    Retry,
}

/// Scheduler that dispatches crawl candidates from tiered ready queues.
#[derive(Debug, Default)]
pub(crate) struct TierScheduler {
    manual: VecDeque<SchedEntry>,
    scheduled: VecDeque<SchedEntry>,
    retry: VecDeque<SchedEntry>,
    residents: HashMap<FeedUrl, Tier>,
    inflights: HashSet<FeedUrl>,
}

impl TierScheduler {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn submit_scheduled(&mut self, input: ScheduledDue) {
        let entry = SchedEntry::from(input);

        if self.is_tracked(entry.feed_url()) {
            debug!(
                feed_url = entry.feed_url().as_str(),
                "scheduled crawl skipped because feed is already tracked"
            );
            return;
        }

        self.residents
            .insert(entry.feed_url().clone(), Tier::Scheduled);
        self.scheduled.push_back(entry);
    }

    fn submit_manual(&mut self, input: ManualRequested) {
        let entry = SchedEntry::from(input);

        if self.inflights.contains(entry.feed_url()) {
            return;
        }

        match self.residents.get(entry.feed_url()).copied() {
            Some(Tier::Manual) => return,
            Some(Tier::Scheduled) => {
                debug_assert!(remove_entry(&mut self.scheduled, entry.feed_url()).is_some());
            }
            Some(Tier::Retry) => {
                debug_assert!(remove_entry(&mut self.retry, entry.feed_url()).is_some());
            }
            None => {}
        }

        self.residents
            .insert(entry.feed_url().clone(), Tier::Manual);
        self.manual.push_back(entry);
    }

    fn submit_retry(&mut self, input: RetryDue) {
        let entry = SchedEntry::from(input);

        if self.inflights.contains(entry.feed_url()) {
            return;
        }

        match self.residents.get(entry.feed_url()).copied() {
            Some(Tier::Manual | Tier::Retry) => return,
            Some(Tier::Scheduled) => {
                debug_assert!(remove_entry(&mut self.scheduled, entry.feed_url()).is_some());
            }
            None => {}
        }

        self.residents.insert(entry.feed_url().clone(), Tier::Retry);
        self.retry.push_back(entry);
    }

    fn is_tracked(&self, feed_url: &FeedUrl) -> bool {
        self.residents.contains_key(feed_url) || self.inflights.contains(feed_url)
    }

    fn pop_ready(&mut self, now: DateTime<Utc>) -> Option<SchedEntry> {
        if self.manual.front().is_some_and(|entry| entry.is_ready(now)) {
            self.manual.pop_front()
        } else if self.retry.front().is_some_and(|entry| entry.is_ready(now)) {
            self.retry.pop_front()
        } else if self
            .scheduled
            .front()
            .is_some_and(|entry| entry.is_ready(now))
        {
            self.scheduled.pop_front()
        } else {
            None
        }
    }
}

impl Scheduler for TierScheduler {
    fn submit(&mut self, input: SchedInput) {
        match input {
            SchedInput::ScheduledDue(input) => {
                self.submit_scheduled(input);
            }
            SchedInput::ManualRequested(input) => {
                self.submit_manual(input);
            }
            SchedInput::RetryDue(input) => {
                self.submit_retry(input);
            }
            SchedInput::CrawlFinished(input) => {
                self.inflights.remove(&input.feed_url);
            }
        }
    }

    fn dispatch(&mut self, cx: DispatchContext) -> DispatchBatch {
        if cx.dispatch_queue_remaining_capacity == 0 {
            return DispatchBatch::empty();
        }

        let Some(entry) = self.pop_ready(cx.now) else {
            return DispatchBatch::empty();
        };

        let feed_url = entry.feed_url().clone();
        debug_assert!(self.residents.remove(&feed_url).is_some());
        self.inflights.insert(feed_url);

        DispatchBatch::one(entry.into_dispatch_entry(cx.now))
    }
}

/// Candidate entry resident in a scheduler implementation's internal queue.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SchedEntry {
    feed_url: FeedUrl,
    trigger: CrawlJobTrigger,
    ready_at: DateTime<Utc>,
}

impl SchedEntry {
    fn feed_url(&self) -> &FeedUrl {
        &self.feed_url
    }

    fn is_ready(&self, now: DateTime<Utc>) -> bool {
        self.ready_at <= now
    }

    fn into_dispatch_entry(self, dispatched_at: DateTime<Utc>) -> DispatchEntry {
        DispatchEntry::new(self.feed_url, self.trigger, dispatched_at)
    }
}

impl From<ScheduledDue> for SchedEntry {
    fn from(value: ScheduledDue) -> Self {
        Self {
            feed_url: value.feed_url,
            trigger: CrawlJobTrigger::PeriodicDue,
            ready_at: value.due_at,
        }
    }
}

impl From<ManualRequested> for SchedEntry {
    fn from(value: ManualRequested) -> Self {
        Self {
            feed_url: value.feed_url,
            trigger: CrawlJobTrigger::ManualRequest,
            ready_at: value.requested_at,
        }
    }
}

impl From<RetryDue> for SchedEntry {
    fn from(value: RetryDue) -> Self {
        Self {
            feed_url: value.feed_url,
            trigger: CrawlJobTrigger::RetryDue,
            ready_at: value.due_at,
        }
    }
}

fn remove_entry(entries: &mut VecDeque<SchedEntry>, feed_url: &FeedUrl) -> Option<SchedEntry> {
    let index = entries
        .iter()
        .position(|entry| entry.feed_url() == feed_url)?;
    entries.remove(index)
}
