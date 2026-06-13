use std::{num::NonZero, pin::Pin, time::Duration};

use tokio::time::{Instant, Sleep};

use crate::{
    application::{InFlight, RequestId, RequestSequence},
    job::{JobFuture, Jobs},
};

use super::feed_events::FeedEventSubscription;

const FOREGROUND_JOB_CONCURRENCY: usize = 90;
const BACKGROUND_JOB_CONCURRENCY: usize = 10;

pub(super) struct DriverRuntime {
    pub(super) jobs: Jobs,
    pub(super) background_jobs: Jobs,
    pub(super) in_flight: InFlight,
    pub(super) idle_timer: Pin<Box<Sleep>>,
}

pub(in crate::application) struct DriverPollers<'a> {
    pub(in crate::application) jobs: &'a mut Jobs,
    pub(in crate::application) background_jobs: &'a mut Jobs,
    pub(in crate::application) feed_events: &'a mut FeedEventSubscription,
    pub(in crate::application) in_flight: &'a mut InFlight,
    pub(in crate::application) idle_timer: &'a mut Pin<Box<Sleep>>,
}

impl DriverRuntime {
    pub(super) fn new(throbber_timer_interval: Duration, idle_timer_interval: Duration) -> Self {
        Self {
            // The secondary rate limit of the GitHub API is 100 concurrent requests,
            // so keep foreground operation concurrency below it.
            jobs: Jobs::new(NonZero::new(FOREGROUND_JOB_CONCURRENCY).unwrap()),
            background_jobs: Jobs::new(NonZero::new(BACKGROUND_JOB_CONCURRENCY).unwrap()),
            in_flight: InFlight::new().with_throbber_timer_interval(throbber_timer_interval),
            idle_timer: Box::pin(tokio::time::sleep(idle_timer_interval)),
        }
    }

    pub(super) fn request_started(&mut self, request_id: RequestId) -> RequestSequence {
        self.in_flight.add(request_id)
    }

    pub(super) fn push_job(&mut self, job: JobFuture) {
        self.jobs.push(job);
    }

    pub(super) fn push_background_job(&mut self, job: JobFuture) {
        self.background_jobs.push(job);
    }

    pub(super) fn reset_throbber(&mut self) {
        self.in_flight.reset_throbber_timer();
        self.in_flight.inc_throbber_step();
    }

    pub(super) fn remove_in_flight(&mut self, request_seq: RequestSequence) {
        self.in_flight.remove(request_seq);
    }

    pub(super) fn clear_idle_timer(&mut self) {
        // https://github.com/tokio-rs/tokio/blob/e53b92a9939565edb33575fff296804279e5e419/tokio/src/time/instant.rs#L62
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + Duration::from_hours(24 * 365 * 30));
    }

    pub(super) fn reset_idle_timer(&mut self, interval: Duration) {
        self.idle_timer.as_mut().reset(Instant::now() + interval);
    }
}
