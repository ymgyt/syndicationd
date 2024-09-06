pub(crate) mod flag {
    use std::time::Duration;

    use synd_stdx::time::humantime::{parse_duration, DurationError};

    pub(crate) fn parse_duration_opt(s: &str) -> Result<Duration, DurationError> {
        parse_duration(s)
    }
}
