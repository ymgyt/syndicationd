use std::{fmt, num::NonZeroU64, time::Duration};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Error returned when a polling interval cannot be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPollingInterval;

impl fmt::Display for InvalidPollingInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("polling interval must be whole seconds greater than zero")
    }
}

impl std::error::Error for InvalidPollingInterval {}

/// Non-zero whole-second interval used by polling crawl policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PollingInterval(NonZeroU64);

impl PollingInterval {
    pub fn duration(self) -> Duration {
        Duration::from_secs(self.as_secs())
    }

    pub fn as_secs(self) -> u64 {
        self.0.get()
    }

    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }
}

impl TryFrom<Duration> for PollingInterval {
    type Error = InvalidPollingInterval;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        if value.subsec_nanos() != 0 {
            return Err(InvalidPollingInterval);
        }

        let Some(seconds) = NonZeroU64::new(value.as_secs()) else {
            return Err(InvalidPollingInterval);
        };

        Ok(Self(seconds))
    }
}

impl<'de> Deserialize<'de> for PollingInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Self::try_from(Duration::from_secs(seconds)).map_err(serde::de::Error::custom)
    }
}

/// Policy for when a feed endpoint may be crawled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PollingPolicy {
    Manual,
    Interval {
        #[serde(rename = "interval_seconds")]
        interval: PollingInterval,
    },
}

impl PollingPolicy {
    pub fn manual() -> Self {
        Self::Manual
    }

    pub fn interval(interval: PollingInterval) -> Self {
        Self::Interval { interval }
    }

    pub fn next_after(self, polled_at: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Manual => None,
            Self::Interval { interval } => Some(add_duration(polled_at, interval)),
        }
    }
}

impl fmt::Display for PollingPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manual => f.write_str("manual"),
            Self::Interval { interval } => write!(f, "interval:{}s", interval.as_secs()),
        }
    }
}

/// Registry-owned crawl policy attached to a subscription or target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlPolicy {
    pub polling: PollingPolicy,
}

impl CrawlPolicy {
    pub fn interval(interval: PollingInterval) -> Self {
        Self {
            polling: PollingPolicy::interval(interval),
        }
    }

    pub fn manual() -> Self {
        Self {
            polling: PollingPolicy::manual(),
        }
    }
}

fn add_duration(time: DateTime<Utc>, interval: PollingInterval) -> DateTime<Utc> {
    chrono::Duration::from_std(interval.duration()).map_or(time, |duration| time + duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(duration: Duration) -> PollingInterval {
        PollingInterval::try_from(duration).unwrap()
    }

    #[test]
    fn polling_interval_rejects_subsecond_duration() {
        assert!(PollingInterval::try_from(Duration::from_millis(500)).is_err());
        assert!(PollingInterval::try_from(Duration::from_millis(1500)).is_err());
    }

    #[test]
    fn polling_interval_serializes_as_seconds() {
        let interval = interval(Duration::from_hours(1));
        let json = serde_json::to_string(&interval).unwrap();

        assert_eq!(json, "3600");
        assert_eq!(
            serde_json::from_str::<PollingInterval>(&json).unwrap(),
            interval
        );
    }

    #[test]
    fn interval_policy_computes_next_poll_time() {
        let polled_at = Utc::now();
        let policy = PollingPolicy::interval(interval(Duration::from_mins(1)));

        assert_eq!(
            policy.next_after(polled_at),
            Some(polled_at + chrono::Duration::seconds(60))
        );
    }

    #[test]
    fn manual_policy_has_no_next_poll_time() {
        let policy = PollingPolicy::manual();

        assert_eq!(policy.next_after(Utc::now()), None);
    }

    #[test]
    fn crawl_policy_serializes_polling_policy() {
        let policy = CrawlPolicy::interval(interval(Duration::from_hours(1)));
        let json = serde_json::to_string(&policy).unwrap();

        assert_eq!(
            json,
            r#"{"polling":{"kind":"interval","interval_seconds":3600}}"#
        );
        assert_eq!(serde_json::from_str::<CrawlPolicy>(&json).unwrap(), policy);
    }
}
