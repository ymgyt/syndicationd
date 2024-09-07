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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse() {
        #[derive(Deserialize)]
        struct Data {
            #[serde(default, deserialize_with = "parse_duration_opt")]
            d: Option<Duration>,
        }

        let s = r#"{"d": "30sec" }"#;
        let data: Data = serde_json::from_str(s).unwrap();
        assert_eq!(data.d, Some(Duration::from_secs(30)));

        let s = r#"{"d": null }"#;
        let data: Data = serde_json::from_str(s).unwrap();
        assert_eq!(data.d, None);
    }
}
