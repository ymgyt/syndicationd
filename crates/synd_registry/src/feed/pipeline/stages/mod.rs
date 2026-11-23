#![allow(
    clippy::unused_async_trait_impl,
    reason = "pure and I/O stages share an asynchronous interface"
)]

mod apply_feed_update;
mod build_feed_update;
mod convert_source_fields;
mod deduplicate_entries;
mod determine_entry_change;
mod identify_entry;
mod load_stored_entries;
mod parse_feed;
pub(super) mod remove_query_params;
mod source_categories;

pub(super) use apply_feed_update::ApplyFeedUpdate;
pub(super) use build_feed_update::BuildFeedUpdate;
pub(super) use convert_source_fields::ConvertSourceFields;
pub(super) use deduplicate_entries::DeduplicateEntries;
pub(super) use determine_entry_change::DetermineEntryChange;
pub(super) use identify_entry::IdentifyEntry;
pub(super) use load_stored_entries::LoadStoredEntries;
pub(super) use parse_feed::ParseFeed;
pub(super) use remove_query_params::RemoveQueryParams;
pub(super) use source_categories::SourceCategories;

use synd_feed::entry::{Entry, SyndEntry};

pub(super) struct EntryWithPrevious {
    observed: Entry,
    previous: Option<SyndEntry>,
}
