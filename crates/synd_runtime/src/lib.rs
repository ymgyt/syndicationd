//! Runtime session and singleton daemon lifecycle.
#![warn(rustdoc::broken_intra_doc_links)]

mod acquisition;
mod api;
mod connection;
mod daemon;
mod database;
mod error;
#[allow(dead_code)]
mod instance;
mod placement;
mod runtime;
mod session;
#[allow(dead_code)]
mod startup;
#[allow(dead_code)]
mod uds;

pub use daemon::{
    Control as DaemonControl, Daemon, DaemonConfig, DaemonExecutable, DaemonLaunchConfig,
    DaemonLaunchInfo, DaemonLaunchLog, RuntimePlacementSummary, ShutdownResult,
    State as DaemonState, Status as DaemonStatus,
};
pub use database::RuntimeDatabase;
pub use error::{Error, Result};
pub use runtime::{ApiClientConfig, Config as RuntimeConfig, Runtime};
pub use session::{
    Config as SessionConfig, Handle as SessionHandle, Requirements as SessionRequirements, Session,
};
pub use synd_api::session::{DaemonSessionConfig, DaemonSessionLeasePolicy};
pub use synd_protocol::CapabilitySet;
pub use synd_protocol::daemon::{DaemonIdleShutdownStatus, DaemonSessionStatus};
