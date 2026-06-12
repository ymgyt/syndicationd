//! Feed lifecycle registry.
#![allow(async_fn_in_trait)]

pub mod command;
pub mod config;
pub mod consumers;
pub mod crawl;
pub mod db;
pub mod entry;
pub mod error;
pub mod event;
pub mod feed;
pub mod query;
pub mod registry;
pub mod subscription;
pub mod timeline;

pub use command::{
    SubscribeFeedCommand, SubscribeFeedOutput, UnsubscribeFeedCommand, UnsubscribeFeedOutput,
};
pub use config::{FeedRegistryConfig, FeedRegistryWorkerConfig};
pub use crawl::worker::{CrawlWorkerFetchConfig, CrawlWorkerPoolConfig, CrawlWorkerQueueConfig};
pub use db::{
    BlobStoreTx, CommitTx, CrawlCompletionTx, CrawlJobQueueTx, CrawlScheduleTx, EntryProjectionTx,
    FeedProjectionTx, FeedRegistryDb, RegistryTx, TimelineProjectionTx,
};
pub use error::{FeedRegistryError, RegistryDbError, RegistryDbResult};
pub use registry::{FeedRegistry, RegistryService};
pub use subscription::{
    FeedSubscriptionAttrs, SubscribeOutcome, SubscriberId, Subscription, SubscriptionKey,
    UnsubscribeOutcome,
};
