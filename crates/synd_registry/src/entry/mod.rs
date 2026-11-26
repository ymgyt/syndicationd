mod reconciliation;

use synd_feed::{
    entry::Entry,
    types::{FeedMeta, SubscriptionLabels},
};

pub(crate) use reconciliation::EntryChange;
pub use reconciliation::{Change, Changes, Entries};

/// A stored entry, its source feed metadata, and the subscriber's labels for that feed.
#[derive(Debug, Clone)]
pub struct SubscribedEntry {
    pub entry: Entry,
    pub feed_meta: FeedMeta,
    pub subscription_labels: SubscriptionLabels,
}
