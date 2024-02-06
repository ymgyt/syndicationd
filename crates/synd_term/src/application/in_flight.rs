use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use tokio::time::Instant;

pub type RequestSequence = u64;

pub enum RequestId {
    FetchEntries,
    FetchSubscription,
}

/// Mangae in flight requests state
pub struct InFlight {
    next_request_sequence: AtomicU64,
    in_flights: HashMap<RequestSequence, InFlightEntry>,
}

impl InFlight {
    pub fn new() -> Self {
        Self {
            next_request_sequence: AtomicU64::new(0),
            in_flights: HashMap::new(),
        }
    }

    pub fn add(&mut self, request_id: RequestId) -> RequestSequence {
        let seq = self.next_request_sequence();
        self.in_flights.insert(
            seq,
            InFlightEntry {
                _start: Instant::now(),
                request_id,
            },
        );
        seq
    }

    pub fn remove(&mut self, seq: RequestSequence) -> Option<RequestId> {
        self.in_flights.remove(&seq).map(|entry| entry.request_id)
    }

    fn next_request_sequence(&self) -> RequestSequence {
        self.next_request_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

struct InFlightEntry {
    // request started at(approximate)
    _start: Instant,
    request_id: RequestId,
}
