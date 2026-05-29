//! Runtime session and singleton daemon lifecycle.
#![warn(rustdoc::broken_intra_doc_links)]

pub mod daemon;

mod capability;
mod database;
mod error;
#[allow(dead_code)]
mod identity;
mod loopback;
mod runtime;
mod session;
#[allow(dead_code)]
mod startup;
#[allow(dead_code)]
mod uds;

pub use capability::CapabilitySet;
pub use daemon::{
    Config as DaemonConfig, Control as DaemonControl, Daemon, LaunchConfig as DaemonLaunchConfig,
    ShutdownResult, State as DaemonState, Status as DaemonStatus,
};
pub use database::RuntimeDatabase;
pub use error::{Error, Result};
pub use runtime::{ApiClientConfig, Config as RuntimeConfig, Runtime};
pub use session::{
    Config as SessionConfig, Handle as SessionHandle, Requirements as SessionRequirements, Session,
};
