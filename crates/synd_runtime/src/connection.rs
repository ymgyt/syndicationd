#[cfg(unix)]
use std::{io::ErrorKind, path::Path};
#[cfg(unix)]
use tokio::net::UnixStream;

use crate::uds::UdsEndpoint;

/// Transport-level connection check for a runtime endpoint.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeEndpointConnector<'a> {
    endpoint: &'a UdsEndpoint,
}

impl<'a> RuntimeEndpointConnector<'a> {
    pub(crate) fn new(endpoint: &'a UdsEndpoint) -> Self {
        Self { endpoint }
    }

    #[cfg(unix)]
    pub(crate) async fn try_connect(&self) -> RuntimeEndpointConnectionStatus {
        match RuntimeEndpointPathState::inspect(self.endpoint.path()) {
            RuntimeEndpointPathState::Missing => return RuntimeEndpointConnectionStatus::Missing,
            RuntimeEndpointPathState::Present => {}
            RuntimeEndpointPathState::Unavailable => {
                return RuntimeEndpointConnectionStatus::Unavailable;
            }
        }

        match UnixStream::connect(self.endpoint.path()).await {
            Ok(_) => RuntimeEndpointConnectionStatus::Connected,
            Err(error) => RuntimeEndpointConnectionStatus::from_io_error(&error),
        }
    }

    #[cfg(not(unix))]
    pub(crate) async fn try_connect(&self) -> RuntimeEndpointConnectionStatus {
        RuntimeEndpointConnectionStatus::UnsupportedTransport
    }
}

/// Result of attempting to connect to a runtime endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeEndpointConnectionStatus {
    Connected,
    Missing,
    Stale,
    Unavailable,
    #[cfg(not(unix))]
    UnsupportedTransport,
}

impl RuntimeEndpointConnectionStatus {
    #[cfg(unix)]
    fn from_io_error(error: &std::io::Error) -> Self {
        match error.kind() {
            ErrorKind::NotFound => Self::Missing,
            ErrorKind::ConnectionRefused => Self::Stale,
            _ => Self::Unavailable,
        }
    }
}

/// Filesystem state observed before connecting to a runtime endpoint.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeEndpointPathState {
    Present,
    Missing,
    Unavailable,
}

#[cfg(unix)]
impl RuntimeEndpointPathState {
    fn inspect(path: &Path) -> Self {
        match path.try_exists() {
            Ok(true) => Self::Present,
            Ok(false) => Self::Missing,
            Err(_) => Self::Unavailable,
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use tokio::net::UnixListener;

    use crate::{
        RuntimeDatabase,
        connection::{RuntimeEndpointConnectionStatus, RuntimeEndpointConnector},
        instance::RuntimeInstance,
        uds::UdsEndpoint,
    };

    mod missing_path {
        use super::*;

        #[tokio::test]
        async fn reports_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let endpoint = endpoint_at(tmp.path());

            let status = RuntimeEndpointConnector::new(&endpoint).try_connect().await;

            assert_eq!(status, RuntimeEndpointConnectionStatus::Missing);
        }
    }

    mod missing_root {
        use super::*;

        #[tokio::test]
        async fn reports_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let db = tmp.path().join("synd.db");
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();
            let endpoint =
                UdsEndpoint::from_instance_id(&tmp.path().join("runtime"), instance.id());

            let status = RuntimeEndpointConnector::new(&endpoint).try_connect().await;

            assert_eq!(status, RuntimeEndpointConnectionStatus::Missing);
        }
    }

    mod active_listener {
        use super::*;

        #[tokio::test]
        async fn reports_connected() {
            let tmp = tempfile::tempdir().unwrap();
            let endpoint = endpoint_at(tmp.path());
            let _listener = UnixListener::bind(endpoint.path()).unwrap();

            let status = RuntimeEndpointConnector::new(&endpoint).try_connect().await;

            assert_eq!(status, RuntimeEndpointConnectionStatus::Connected);
        }
    }

    mod orphan_socket {
        use super::*;

        #[tokio::test]
        async fn reports_stale() {
            let tmp = tempfile::tempdir().unwrap();
            let endpoint = endpoint_at(tmp.path());
            let listener = UnixListener::bind(endpoint.path()).unwrap();
            drop(listener);

            let status = RuntimeEndpointConnector::new(&endpoint).try_connect().await;

            assert_eq!(status, RuntimeEndpointConnectionStatus::Stale);
        }
    }

    fn endpoint_at(root: &std::path::Path) -> UdsEndpoint {
        let db = root.join("synd.db");
        let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(db)).unwrap();

        UdsEndpoint::from_instance_id(root, instance.id())
    }
}
