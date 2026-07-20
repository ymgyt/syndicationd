mod content;
pub use content::Content;

#[allow(
    clippy::module_inception,
    reason = "entry.rs owns the primary Entry value exposed by this first-class module"
)]
mod entry;
pub use entry::Entry;

mod id;
pub(crate) use id::feed_rs_missing_id_marker;
pub use id::{EntryId, EntryIdError};
