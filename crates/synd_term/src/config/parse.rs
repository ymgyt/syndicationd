pub(crate) mod de {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer};

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
    use humantime::DurationError;
    use std::time::Duration;

    pub(crate) fn parse_duration_opt(s: &str) -> Result<Duration, DurationError> {
        humantime::parse_duration(s)
    }
}