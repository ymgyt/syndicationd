use std::time::Duration;

use serde::{Deserialize, Deserializer};

use crate::time::humantime;

pub fn parse_duration_opt<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
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
