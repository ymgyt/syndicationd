//! Feed lifecycle registry.
#![allow(async_fn_in_trait)]

pub mod command;
pub mod config;
pub mod consumers;
pub mod crawl;
pub mod db;
pub mod error;
pub mod event;
pub mod registry;
pub mod runtime;
pub mod subscriber;
pub mod subscription;
pub mod view;

pub use command::{
    SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
};
pub use config::FeedRegistryConfig;
pub use db::{FeedRegistryDb, RegistryDbTransaction};
pub use error::{FeedRegistryError, RegistryDbError, RegistryDbResult};
pub use registry::FeedRegistry;
pub use runtime::{FeedRegistryRuntime, RuntimeEventSubmitter, RuntimeFeedRegistry};
pub use subscriber::SubscriberId;
pub use subscription::{Subscription, SubscriptionAnnotations, SubscriptionKey};
