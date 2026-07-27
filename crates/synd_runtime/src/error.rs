use std::{path::PathBuf, process::ExitStatus};

use synd_protocol::CapabilitySet;

use thiserror::Error;

use crate::DaemonLaunchInfo;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    RegistryDb(#[from] synd_registry::RegistryDbError),

    #[error(transparent)]
    Migration(#[from] synd_persistence::sqlite::MigrationError),

    #[error(transparent)]
    Api(Box<synd_client::SyndApiError>),

    #[error(transparent)]
    RuntimeApi(Box<synd_api::Error>),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("{context} endpoint unavailable: {}", endpoint.display())]
    EndpointUnavailable {
        context: &'static str,
        endpoint: PathBuf,
    },

    #[error("{context} transport unsupported")]
    UnsupportedTransport { context: &'static str },

    #[error(
        "runtime daemon at {} is missing required session capabilities: {missing_capabilities}",
        endpoint.display()
    )]
    MissingSessionCapabilities {
        endpoint: PathBuf,
        missing_capabilities: CapabilitySet,
    },

    #[error("refusing to remove non-socket runtime endpoint {}", path.display())]
    NonSocketEndpoint { path: PathBuf },

    #[error(
        "daemon exited before endpoint became ready: {status}; daemon log: {}",
        launch.log().display()
    )]
    DaemonExitedBeforeReady {
        status: ExitStatus,
        launch: DaemonLaunchInfo,
    },

    #[error(
        "timed out waiting for launched daemon endpoint {} to become ready; daemon log: {}",
        endpoint.display(),
        launch.log().display()
    )]
    DaemonEndpointReadyTimeout {
        endpoint: PathBuf,
        launch: DaemonLaunchInfo,
    },

    #[error("timed out waiting for daemon endpoint {} to become ready", endpoint.display())]
    EndpointReadyTimeout { endpoint: PathBuf },

    #[error("timed out waiting for daemon endpoint {} to stop", endpoint.display())]
    EndpointStopTimeout { endpoint: PathBuf },

    #[error("daemon claim lock is already held at {}", path.display())]
    DaemonClaimLockAlreadyHeld { path: PathBuf },

    #[error("refusing force shutdown: {reason}")]
    ForceShutdownRefused { reason: String },

    #[error("timed out force stopping daemon process {pid}")]
    ForceShutdownTimeout { pid: u32 },

    #[error("runtime daemon is incompatible at {}; {suggestion}", endpoint.display())]
    IncompatibleRuntimeDaemon {
        endpoint: PathBuf,
        suggestion: String,
    },

    #[error("failed to stop incompatible runtime daemon at {}; {suggestion}", endpoint.display())]
    IncompatibleRuntimeDaemonShutdown {
        endpoint: PathBuf,
        suggestion: String,
        #[source]
        source: Box<synd_client::SyndApiError>,
    },

    #[error("timed out stopping incompatible runtime daemon at {}; {suggestion}", endpoint.display())]
    IncompatibleRuntimeDaemonStopTimeout {
        endpoint: PathBuf,
        suggestion: String,
    },
}

impl From<synd_client::SyndApiError> for Error {
    fn from(source: synd_client::SyndApiError) -> Self {
        Self::Api(Box::new(source))
    }
}

impl From<synd_api::Error> for Error {
    fn from(source: synd_api::Error) -> Self {
        Self::RuntimeApi(Box::new(source))
    }
}
