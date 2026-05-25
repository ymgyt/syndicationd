use std::time::Duration;

pub type DurationError = humantime::DurationError;

/// Parse the string representation of a duration, such as "30s".
pub fn parse_duration(s: &str) -> Result<Duration, DurationError> {
    humantime::parse_duration(s)
}
