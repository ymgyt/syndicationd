//! Current registry implementation kept behind the public `FeedRegistry` facade.
//!
//! This module owns the old synchronous registry behavior: subscription writes,
//! refresh execution, refresh planning, provider access, and client notifications.
//! Event-flow workers/projectors should live outside this module.

mod bridge;

pub mod executor;
pub mod model;
pub mod planner;
pub mod provider;
pub mod reconciler;

pub use bridge::LegacyBridge;
pub use executor::{RefreshExecutor, RefreshExecutorHandle};
pub use planner::{ReconcilePlan, RefreshPlanner, RefreshRequestDecision, RefreshRequestPolicy};
pub use provider::{FeedProvider, FeedProviderError, FetchedFeed, SyndFeedProvider};
pub use reconciler::Reconciler;
