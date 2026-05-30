//! Shared wire protocol contracts for syndicationd client/server boundaries.
#![warn(rustdoc::broken_intra_doc_links)]

pub mod capability;
pub mod session;

pub use capability::CapabilitySet;
