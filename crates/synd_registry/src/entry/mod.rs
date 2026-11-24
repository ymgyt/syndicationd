mod reconciliation;

use synd_feed::{
    entry::Entry,
    types::{Annotated, FeedMeta},
};

pub(crate) use reconciliation::EntryChange;
pub use reconciliation::{Change, Changes, Entries};

/// A stored entry and its source feed as seen by a subscriber.
#[derive(Debug, Clone)]
pub struct SubscribedEntry {
    pub entry: Entry,
    pub feed_meta: Annotated<FeedMeta>,
}
