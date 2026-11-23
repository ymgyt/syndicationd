use std::collections::HashSet;

use synd_feed::{
    entry::{EntryId, EntryIdMismatch},
    types::FeedMeta,
};
use thiserror::Error;

use super::FeedSource;
use crate::entry::Changes;

/// Atomic state transition derived from one accepted feed observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedUpdate {
    pub(super) source: FeedSource,
    pub(super) meta: FeedMeta,
    pub(super) membership: Vec<EntryId>,
    pub(super) preserve_membership: HashSet<EntryId>,
    pub(super) entry_changes: Changes,
}

impl FeedUpdate {
    pub(crate) fn source(&self) -> &FeedSource {
        &self.source
    }

    pub(crate) fn meta(&self) -> &FeedMeta {
        &self.meta
    }

    pub(crate) fn membership(&self) -> &[EntryId] {
        &self.membership
    }

    /// IDs whose existing membership must survive a conflicting observation.
    pub(crate) fn preserve_membership(&self) -> &HashSet<EntryId> {
        &self.preserve_membership
    }

    pub(crate) fn entry_changes(&self) -> &Changes {
        &self.entry_changes
    }
}

/// Deterministic failure while deciding an accepted feed observation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FeedUpdateError {
    #[error(transparent)]
    EntryIdMismatch(#[from] EntryIdMismatch),
}
