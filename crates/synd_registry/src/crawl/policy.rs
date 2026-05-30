use std::{collections::HashMap, fmt, num::NonZeroU64, time::Duration};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use synd_feed::types::FeedUrl;

use crate::subscription::Subscription;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidRefreshInterval;

impl fmt::Display for InvalidRefreshInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("refresh interval must be whole seconds greater than zero")
    }
}

impl std::error::Error for InvalidRefreshInterval {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RefreshInterval(NonZeroU64);

impl RefreshInterval {
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

impl TryFrom<Duration> for RefreshInterval {
    type Error = InvalidRefreshInterval;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        if value.subsec_nanos() != 0 {
            return Err(InvalidRefreshInterval);
        }

        let Some(seconds) = NonZeroU64::new(value.as_secs()) else {
            return Err(InvalidRefreshInterval);
        };

        Ok(Self(seconds))
    }
}

impl<'de> Deserialize<'de> for RefreshInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Self::try_from(Duration::from_secs(seconds)).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshSchedule {
    Manual,
    Interval(RefreshInterval),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshPolicy {
    pub schedule: RefreshSchedule,
}

impl RefreshPolicy {
    pub fn interval(interval: RefreshInterval) -> Self {
        Self {
            schedule: RefreshSchedule::Interval(interval),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveRefreshPolicy {
    pub schedule: RefreshSchedule,
}

impl EffectiveRefreshPolicy {
    pub fn from_subscriptions<'a>(
        subscriptions: impl IntoIterator<Item = &'a Subscription>,
    ) -> Option<Self> {
        let mut has_subscription = false;
        let mut interval = None;

        for subscription in subscriptions {
            has_subscription = true;
            if let RefreshSchedule::Interval(candidate) = subscription.refresh_policy.schedule {
                interval = Some(
                    interval.map_or(candidate, |current: RefreshInterval| current.min(candidate)),
                );
            }
        }

        has_subscription.then_some(Self {
            schedule: interval.map_or(RefreshSchedule::Manual, RefreshSchedule::Interval),
        })
    }

    pub fn next_after(self, refreshed_at: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self.schedule {
            RefreshSchedule::Manual => None,
            RefreshSchedule::Interval(interval) => Some(add_duration(refreshed_at, interval)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredFeedRefresh {
    pub feed_url: FeedUrl,
    pub policy: EffectiveRefreshPolicy,
}

impl DesiredFeedRefresh {
    pub fn from_subscriptions(subscriptions: Vec<Subscription>) -> Vec<Self> {
        let mut by_feed_url = HashMap::<FeedUrl, Vec<Subscription>>::new();
        for subscription in subscriptions {
            by_feed_url
                .entry(subscription.feed_url.clone())
                .or_default()
                .push(subscription);
        }

        let mut desired = by_feed_url
            .into_iter()
            .filter_map(|(feed_url, subscriptions)| {
                EffectiveRefreshPolicy::from_subscriptions(&subscriptions)
                    .map(|policy| Self { feed_url, policy })
            })
            .collect::<Vec<_>>();
        desired.sort_unstable_by(|a, b| a.feed_url.as_str().cmp(b.feed_url.as_str()));
        desired
    }
}

fn add_duration(time: DateTime<Utc>, interval: RefreshInterval) -> DateTime<Utc> {
    chrono::Duration::from_std(interval.duration()).map_or(time, |duration| time + duration)
}

#[cfg(test)]
mod tests {
    use synd_feed::types::FeedUrl;

    use super::*;
    use crate::subscriber::SubscriberId;

    fn interval(duration: Duration) -> RefreshInterval {
        RefreshInterval::try_from(duration).unwrap()
    }

    fn subscription(refresh_policy: RefreshPolicy) -> Subscription {
        let now = Utc::now();
        Subscription {
            subscriber_id: SubscriberId::new("local"),
            feed_url: FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            requirement: None,
            category: None,
            refresh_policy,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn refresh_interval_rejects_subsecond_duration() {
        assert!(RefreshInterval::try_from(Duration::from_millis(500)).is_err());
        assert!(RefreshInterval::try_from(Duration::from_millis(1500)).is_err());
    }

    #[test]
    fn refresh_interval_serializes_as_seconds() {
        let interval = interval(Duration::from_hours(1));
        let json = serde_json::to_string(&interval).unwrap();

        assert_eq!(json, "3600");
        assert_eq!(
            serde_json::from_str::<RefreshInterval>(&json).unwrap(),
            interval
        );
    }

    #[test]
    fn effective_refresh_policy_uses_shortest_interval() {
        let subscriptions = [
            subscription(RefreshPolicy::interval(interval(Duration::from_hours(1)))),
            subscription(RefreshPolicy::interval(interval(Duration::from_mins(10)))),
            subscription(RefreshPolicy {
                schedule: RefreshSchedule::Manual,
            }),
        ];

        let policy = EffectiveRefreshPolicy::from_subscriptions(&subscriptions).unwrap();

        assert_eq!(
            policy.schedule,
            RefreshSchedule::Interval(interval(Duration::from_mins(10)))
        );
    }

    #[test]
    fn effective_refresh_policy_is_manual_when_all_subscriptions_are_manual() {
        let subscriptions = [subscription(RefreshPolicy {
            schedule: RefreshSchedule::Manual,
        })];

        let policy = EffectiveRefreshPolicy::from_subscriptions(&subscriptions).unwrap();

        assert_eq!(policy.schedule, RefreshSchedule::Manual);
    }

    #[test]
    fn desired_feed_refresh_groups_subscriptions_by_feed() {
        let mut other = subscription(RefreshPolicy::interval(interval(Duration::from_mins(5))));
        other.subscriber_id = SubscriberId::new("other");
        let desired = DesiredFeedRefresh::from_subscriptions(vec![
            subscription(RefreshPolicy::interval(interval(Duration::from_hours(1)))),
            other,
        ]);

        assert_eq!(desired.len(), 1);
        assert_eq!(
            desired[0].policy.schedule,
            RefreshSchedule::Interval(interval(Duration::from_mins(5)))
        );
    }
}
