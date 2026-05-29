use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryNotification {
    TimelineChanged(TimelineChanged),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineChanged {
    pub changed_at: DateTime<Utc>,
    pub affected_feeds: AffectedFeeds,
}

impl TimelineChanged {
    pub fn for_feed(feed_url: FeedUrl, changed_at: DateTime<Utc>) -> Self {
        Self {
            changed_at,
            affected_feeds: AffectedFeeds::Known(vec![feed_url]),
        }
    }

    pub fn for_feeds(feed_urls: Vec<FeedUrl>, changed_at: DateTime<Utc>) -> Self {
        Self {
            changed_at,
            affected_feeds: AffectedFeeds::Known(feed_urls),
        }
    }

    pub fn unknown(changed_at: DateTime<Utc>) -> Self {
        Self {
            changed_at,
            affected_feeds: AffectedFeeds::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AffectedFeeds {
    Unknown,
    Known(Vec<FeedUrl>),
}

#[derive(Clone)]
pub struct RegistryNotificationPublisher {
    sender: broadcast::Sender<RegistryNotification>,
}

pub struct RegistryNotificationSubscriber {
    receiver: broadcast::Receiver<RegistryNotification>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryNotificationRecvError {
    Closed,
    Lagged(u64),
}

impl RegistryNotificationPublisher {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> RegistryNotificationSubscriber {
        RegistryNotificationSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, notification: RegistryNotification) -> usize {
        self.sender.send(notification).unwrap_or_default()
    }
}

impl Default for RegistryNotificationPublisher {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl RegistryNotificationSubscriber {
    pub async fn recv(&mut self) -> Result<RegistryNotification, RegistryNotificationRecvError> {
        self.receiver.recv().await.map_err(|err| match err {
            broadcast::error::RecvError::Closed => RegistryNotificationRecvError::Closed,
            broadcast::error::RecvError::Lagged(skipped) => {
                RegistryNotificationRecvError::Lagged(skipped)
            }
        })
    }
}
