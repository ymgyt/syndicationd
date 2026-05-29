//! Feed lifecycle registry.
#![allow(async_fn_in_trait)]

pub mod command;
pub mod consumers;
pub mod crawl;
pub mod db;
pub mod error;
pub mod event;
pub mod legacy;
pub mod registry;
pub mod subscriber;
pub mod subscription;

pub use command::{RegistryCommand, Subscribe, Unsubscribe};
pub use db::{FeedRegistryDb, RegistryDbTransaction};
pub use error::{FeedRegistryError, RegistryDbError, RegistryDbResult};
pub use registry::FeedRegistry;
pub use subscriber::SubscriberId;
pub use subscription::Subscription;
