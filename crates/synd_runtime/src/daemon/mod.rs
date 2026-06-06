#[cfg(unix)]
mod claim;
mod control;
mod launch;
mod service;
mod status;

#[cfg(unix)]
pub(crate) use claim::{
    DaemonClaim, DaemonClaimLockAcquirer, DaemonClaimOwner, SignalTarget, remove_stale_claim,
    wait_until_claim_released,
};
pub use control::Control;
pub use launch::{DaemonExecutable, DaemonLaunchConfig, DaemonLaunchInfo, DaemonLaunchLog};
pub(crate) use launch::{DaemonHandle, DaemonLauncher};
pub use service::{Daemon, DaemonConfig};
pub use status::{PlacementSummary, ShutdownResult, State, Status};
