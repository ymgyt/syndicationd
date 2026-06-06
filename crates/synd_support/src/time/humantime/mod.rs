use std::{fmt, time::Duration};

mod parse;

pub use parse::{DurationError, parse_duration};

pub mod de;

/// Human-readable representation of a `Duration` used at text boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HumanDuration(Duration);

impl HumanDuration {
    pub fn duration(self) -> Duration {
        self.0
    }
}

impl From<Duration> for HumanDuration {
    fn from(duration: Duration) -> Self {
        Self(duration)
    }
}

impl From<HumanDuration> for Duration {
    fn from(duration: HumanDuration) -> Self {
        duration.duration()
    }
}

impl fmt::Display for HumanDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        humantime::format_duration(self.0).fmt(f)
    }
}

impl From<HumanDuration> for String {
    fn from(duration: HumanDuration) -> Self {
        duration.to_string()
    }
}
