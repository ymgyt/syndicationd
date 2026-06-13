mod clock;
#[cfg(feature = "humantime")]
pub mod humantime;

pub use clock::{Clock, SystemClock};

#[cfg(any(test, feature = "mock"))]
pub use clock::FakeClock;
