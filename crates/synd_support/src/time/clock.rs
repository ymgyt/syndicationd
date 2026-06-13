use chrono::{DateTime, Utc};

/// Source of wall-clock time for components that must observe real time.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> DateTime<Utc>;
}

/// Clock backed by the system wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(any(test, feature = "mock"))]
mod fake {
    use std::sync::{Arc, Mutex};

    use chrono::{DateTime, Utc};

    use super::Clock;

    /// Test clock whose current time is controlled by the caller.
    #[derive(Debug, Clone)]
    pub struct FakeClock {
        now: Arc<Mutex<DateTime<Utc>>>,
    }

    impl FakeClock {
        pub fn new(now: DateTime<Utc>) -> Self {
            Self {
                now: Arc::new(Mutex::new(now)),
            }
        }

        pub fn set(&self, now: DateTime<Utc>) {
            *self.now.lock().expect("fake clock mutex poisoned") = now;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.now.lock().expect("fake clock mutex poisoned")
        }
    }
}

#[cfg(any(test, feature = "mock"))]
pub use fake::FakeClock;
