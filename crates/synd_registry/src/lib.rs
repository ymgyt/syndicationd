//! Feed lifecycle registry.
#![allow(async_fn_in_trait)]

pub mod command;
pub mod config;
pub mod consumers;
pub mod crawl;
pub mod db;
pub mod error;
pub mod event;
pub mod query;
pub mod registry;
pub mod runtime;
pub mod subscription;

pub use command::{
    SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
};
pub use config::FeedRegistryConfig;
pub use db::{CommitTx, FeedRegistryDb, RegistryTx};
pub use error::{FeedRegistryError, RegistryDbError, RegistryDbResult};
pub use registry::FeedRegistry;
pub use subscription::{SubscriberId, Subscription, SubscriptionKey};
