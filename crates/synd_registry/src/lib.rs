//! Feed lifecycle registry.
#![allow(async_fn_in_trait)]

pub mod api;
pub mod command;
pub mod config;
pub mod crawl;
pub mod db;
pub mod entry;
pub mod error;
pub mod event;
pub mod feed;
mod handler;
#[cfg(any(test, feature = "test"))]
pub mod in_memory;
pub mod query;
pub mod registry;
pub mod subscription;
pub mod timeline;

pub use command::{
    RequestCrawlCommand, RequestCrawlOutput, SubscribeFeedCommand, SubscribeFeedOutput,
    UnsubscribeFeedCommand, UnsubscribeFeedOutput,
};
pub use config::{CrawlDispatchConfig, FeedRegistryConfig, FeedRegistryWorkerConfig};
pub use crawl::request::{CrawlRequestReject, RequestCrawlOutcome};
pub use crawl::worker::{CrawlWorkerFetchConfig, CrawlWorkerPoolConfig, CrawlWorkerQueueConfig};
pub use db::{
    BlobStore, CommitTx, CrawlStateStore, CrawlTargetStore, EntryStore, FeedRegistryDb, FeedStore,
    SubscriptionStore, TimelineStore,
};
pub use error::{FeedRegistryError, RegistryDbError, RegistryDbResult};
#[cfg(any(test, feature = "test"))]
pub use in_memory::{InMemoryFeedRegistryDb, InMemoryRegistryTx};
pub use registry::FeedRegistry;
pub use subscription::{
    FeedSubscriptionAttrs, SubscribeOutcome, SubscriberId, Subscription, SubscriptionKey,
    UnsubscribeOutcome,
};
