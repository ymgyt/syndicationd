//! Shared support utilities for syndicationd crates.
#![warn(rustdoc::broken_intra_doc_links)]

#[cfg(feature = "byte")]
pub mod byte;
#[cfg(feature = "conf")]
pub mod conf;
pub mod fs;
pub mod io;
#[cfg(feature = "o11y")]
pub mod o11y;
pub mod prelude;
pub mod time;
