//! Feed lifecycle registry.
#![allow(async_fn_in_trait)]

pub mod error;
pub mod event;
pub mod executor;
pub mod model;
pub mod planner;
pub mod provider;
pub mod reconciler;
pub mod registry;
pub mod store;

pub use error::{FeedRegistryError, StoreError, StoreResult};
pub use event::{RegistryEventPublisher, RegistryEventRecvError, RegistryEventSubscriber};
pub use executor::{RefreshExecutor, RefreshExecutorHandle};
pub use model::*;
pub use planner::{ReconcilePlan, RefreshPlanner, RefreshRequestDecision, RefreshRequestPolicy};
pub use provider::{FeedProvider, FeedProviderError, FetchedFeed};
pub use reconciler::Reconciler;
pub use registry::FeedRegistry;
pub use store::{FeedRegistryStore, RegistryTransaction};
