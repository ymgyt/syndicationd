pub(crate) mod de {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer};
    use synd_stdx::time::humantime;

    pub(crate) fn parse_duration_opt<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<String>::deserialize(deserializer)? {
            Some(duration) => match humantime::parse_duration(&duration) {
                Ok(duration) => Ok(Some(duration)),
                Err(err) => Err(serde::de::Error::custom(err)),
            },
            None => Ok(None),
        }
    }
}

pub(crate) mod flag {
    use std::time::Duration;

    use synd_stdx::time::humantime::{parse_duration, DurationError};

    pub(crate) fn parse_duration_opt(s: &str) -> Result<Duration, DurationError> {
        parse_duration(s)
    }
}
