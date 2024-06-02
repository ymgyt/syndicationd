use std::{
    collections::HashMap,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use tokio::time::{Instant, Sleep};

pub type RequestSequence = u64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestId {
    DeviceFlowDeviceAuthorize,
    DeviceFlowPollAccessToken,
    FetchEntries,
    FetchSubscription,
    SubscribeFeed,
    UnsubscribeFeed,
}

/// Mangae in flight requests state
pub struct InFlight {
    next_request_sequence: AtomicU64,
    in_flights: HashMap<RequestSequence, InFlightEntry>,
    throbber_timer: Pin<Box<Sleep>>,
    throbber_step: i8,
    throbber_timer_interval: Duration,
}

impl InFlight {
    pub fn new() -> Self {
        Self {
            next_request_sequence: AtomicU64::new(0),
            in_flights: HashMap::new(),
            throbber_timer: Box::pin(tokio::time::sleep(Duration::from_secs(3600 * 24))),
            throbber_step: 0,
            throbber_timer_interval: Duration::from_millis(250),
        }
    }

    #[must_use]
    pub fn with_throbber_timer_interval(self, interval: Duration) -> Self {
        Self {
            throbber_timer_interval: interval,
            ..self
        }
    }

    pub fn recent_in_flight(&self) -> Option<RequestId> {
        self.in_flights
            .iter()
            .max_by_key(|(_, entry)| entry.start)
            .map(|(_, entry)| entry.request_id)
    }

    pub async fn throbber_timer(&mut self) {
        self.throbber_timer.as_mut().await;
    }

    pub fn reset_throbber_timer(&mut self) {
        self.throbber_timer
            .as_mut()
            .reset(Instant::now() + self.throbber_timer_interval);
    }

    pub fn inc_throbber_step(&mut self) {
        self.throbber_step = self.throbber_step.wrapping_add(1);
    }

    pub fn throbber_step(&self) -> i8 {
        self.throbber_step
    }

    pub fn add(&mut self, request_id: RequestId) -> RequestSequence {
        let seq = self.next_request_sequence();
        self.in_flights.insert(
            seq,
            InFlightEntry {
                start: Instant::now(),
                request_id,
            },
        );

        self.reset_throbber_timer();

        seq
    }

    pub fn remove(&mut self, seq: RequestSequence) -> Option<RequestId> {
        let req_id = self.in_flights.remove(&seq).map(|entry| entry.request_id);

        if self.in_flights.is_empty() {
            self.throbber_timer
                .as_mut()
                .reset(Instant::now() + Duration::from_secs(3600 * 24));
        }

        req_id
    }

    fn next_request_sequence(&self) -> RequestSequence {
        self.next_request_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

struct InFlightEntry {
    // request started at(approximate)
    start: Instant,
    request_id: RequestId,
}
