pub(crate) mod flag {
    use std::time::Duration;

    use synd_stdx::time::humantime::{DurationError, parse_duration};

    pub(crate) fn parse_duration_opt(s: &str) -> Result<Duration, DurationError> {
        parse_duration(s)
    }
}
