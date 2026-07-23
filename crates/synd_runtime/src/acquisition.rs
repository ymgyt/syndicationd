use std::{
    path::Path,
    process::ExitStatus,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

use synd_protocol::session::{OpenSessionErrorCode, OpenSessionErrorResponse, OpenSessionRequest};
use tokio::time::sleep;
use tracing::{debug, info};

use crate::{
    Error, Result, Runtime, RuntimeConfig, Session,
    connection::{RuntimeEndpointConnectionStatus, RuntimeEndpointConnector},
    daemon::{DaemonHandle, DaemonLauncher},
    placement::PlacementSpec,
    startup::{StartupLock, StartupLockAcquirer, StartupLockAcquisition},
};

const ENDPOINT_WAIT_INTERVAL: Duration = Duration::from_millis(50);

/// Controls the ordered states required to acquire a runtime session.
pub(crate) struct SessionAcquisition<'a> {
    runtime: &'a Runtime,
}

impl<'a> SessionAcquisition<'a> {
    pub(crate) fn new(runtime: &'a Runtime) -> Self {
        Self { runtime }
    }

    pub(crate) async fn acquire(self) -> Result<Session> {
        let endpoint = Endpoint::probe(self.runtime.placement().clone()).await;
        endpoint.trace("Resolved runtime session acquisition state");

        match endpoint {
            Endpoint::Connected(connected) => self.open(connected, DaemonAction::Existing).await,
            Endpoint::Missing(missing) => self.start(missing).await,
            Endpoint::Stale(stale) => self.recover_stale(stale).await,
            Endpoint::Unavailable(unavailable) => unavailable.fail("runtime endpoint"),
            #[cfg(not(unix))]
            Endpoint::Unsupported(unsupported) => unsupported.fail("runtime endpoint"),
        }
    }

    async fn open(&self, connected: Connected, daemon_action: DaemonAction) -> Result<Session> {
        match connected.open(self.runtime.config(), daemon_action).await? {
            SessionAttempt::Opened(session) => Ok(*session),
            SessionAttempt::Incompatible(incompatible) => self.recover(incompatible).await,
        }
    }

    async fn start(&self, missing: Missing) -> Result<Session> {
        match Startup::acquire(missing.placement())? {
            Startup::Held(held) => self.after_lock(&held).await,
            Startup::Busy(busy) => self.after_busy(busy).await,
            #[cfg(not(unix))]
            Startup::Unsupported(unsupported) => unsupported.fail("runtime startup lock"),
        }
    }

    async fn recover_stale(&self, stale: Stale) -> Result<Session> {
        match Startup::acquire(stale.placement())? {
            Startup::Held(held) => self.after_lock(&held).await,
            Startup::Busy(busy) => self.after_busy(busy).await,
            #[cfg(not(unix))]
            Startup::Unsupported(unsupported) => unsupported.fail("runtime startup lock"),
        }
    }

    async fn recover(&self, incompatible: Incompatible) -> Result<Session> {
        match Startup::acquire(incompatible.placement())? {
            Startup::Held(held) => self.replace(incompatible, &held).await,
            Startup::Busy(busy) => self.after_busy(busy).await,
            #[cfg(not(unix))]
            Startup::Unsupported(unsupported) => unsupported.fail("runtime startup lock"),
        }
    }

    async fn after_lock(&self, held: &Held) -> Result<Session> {
        let endpoint = Endpoint::probe(self.runtime.placement().clone()).await;
        endpoint.trace("Resolved runtime session acquisition state after startup lock");

        match endpoint {
            Endpoint::Connected(connected) => {
                match connected
                    .open(self.runtime.config(), DaemonAction::Existing)
                    .await?
                {
                    SessionAttempt::Opened(session) => Ok(*session),
                    SessionAttempt::Incompatible(incompatible) => {
                        self.replace(incompatible, held).await
                    }
                }
            }
            Endpoint::Missing(missing) => self.launch(missing, held, DaemonAction::Launched).await,
            Endpoint::Stale(stale) => {
                let missing = stale.cleanup(held)?;
                self.launch(missing, held, DaemonAction::Recovered).await
            }
            Endpoint::Unavailable(unavailable) => unavailable.fail("runtime endpoint"),
            #[cfg(not(unix))]
            Endpoint::Unsupported(unsupported) => unsupported.fail("runtime endpoint"),
        }
    }

    async fn after_busy(&self, busy: Busy) -> Result<Session> {
        let connected = busy.wait(self.runtime.config()).await?;
        match connected
            .open(self.runtime.config(), DaemonAction::Waited)
            .await?
        {
            SessionAttempt::Opened(session) => Ok(*session),
            SessionAttempt::Incompatible(incompatible) => incompatible.fail(),
        }
    }

    async fn replace(&self, incompatible: Incompatible, held: &Held) -> Result<Session> {
        let stopped = incompatible.stop(self.runtime.config(), held).await?;
        let missing = match stopped {
            Stopped::Missing(missing) => missing,
            Stopped::Stale(stale) => stale.cleanup(held)?,
        };

        self.launch(missing, held, DaemonAction::Replaced).await
    }

    async fn launch(
        &self,
        missing: Missing,
        held: &Held,
        daemon_action: DaemonAction,
    ) -> Result<Session> {
        let connected = missing.start(self.runtime.config(), held).await?;
        match connected.open(self.runtime.config(), daemon_action).await? {
            SessionAttempt::Opened(session) => Ok(*session),
            SessionAttempt::Incompatible(incompatible) => incompatible.fail(),
        }
    }
}

#[derive(Debug, Clone)]
enum Endpoint {
    Connected(Connected),
    Missing(Missing),
    Stale(Stale),
    Unavailable(Unavailable),
    #[cfg(not(unix))]
    Unsupported(UnsupportedEndpoint),
}

impl Endpoint {
    async fn probe(placement: PlacementSpec) -> Self {
        let connection = RuntimeEndpointConnector::new(placement.endpoint())
            .try_connect()
            .await;

        Self::from_connection(placement, connection)
    }

    fn from_connection(
        placement: PlacementSpec,
        connection: RuntimeEndpointConnectionStatus,
    ) -> Self {
        match connection {
            RuntimeEndpointConnectionStatus::Connected => Self::Connected(Connected { placement }),
            RuntimeEndpointConnectionStatus::Missing => Self::Missing(Missing { placement }),
            RuntimeEndpointConnectionStatus::Stale => Self::Stale(Stale { placement }),
            RuntimeEndpointConnectionStatus::Unavailable => {
                Self::Unavailable(Unavailable { placement })
            }
            #[cfg(not(unix))]
            RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                Self::Unsupported(UnsupportedEndpoint { placement })
            }
        }
    }

    fn trace(&self, message: &'static str) {
        let placement = self.placement();
        debug!(
            runtime_root = %placement.root().path().display(),
            runtime_instance_id = %placement.instance().id(),
            runtime_database = %placement.instance().canonical_database_path().display(),
            runtime_endpoint = %placement.endpoint().path().display(),
            runtime_startup_lock = %placement.startup_lock_path().path().display(),
            runtime_endpoint_state = self.name(),
            message
        );
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Connected(_) => "connected",
            Self::Missing(_) => "missing",
            Self::Stale(_) => "stale",
            Self::Unavailable(_) => "unavailable",
            #[cfg(not(unix))]
            Self::Unsupported(_) => "unsupported",
        }
    }

    fn placement(&self) -> &PlacementSpec {
        match self {
            Self::Connected(state) => &state.placement,
            Self::Missing(state) => &state.placement,
            Self::Stale(state) => &state.placement,
            Self::Unavailable(state) => &state.placement,
            #[cfg(not(unix))]
            Self::Unsupported(state) => &state.placement,
        }
    }
}

#[derive(Debug, Clone)]
struct Connected {
    placement: PlacementSpec,
}

#[derive(Debug, Clone, Copy)]
enum DaemonAction {
    Existing,
    Launched,
    Recovered,
    Replaced,
    Waited,
}

impl DaemonAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Existing => "existing",
            Self::Launched => "launched",
            Self::Recovered => "recovered",
            Self::Replaced => "replaced",
            Self::Waited => "waited",
        }
    }
}

impl Connected {
    async fn open(
        self,
        config: &RuntimeConfig,
        daemon_action: DaemonAction,
    ) -> Result<SessionAttempt> {
        let client = daemon_client(config, &self.placement)?;
        let required_capabilities = config.requirements().required_capabilities().clone();
        debug!(
            runtime_endpoint = %self.placement.endpoint().path().display(),
            required_capabilities = %required_capabilities,
            "Opening daemon session"
        );
        match client
            .open_session(OpenSessionRequest::new(required_capabilities))
            .await
        {
            Ok(session) => {
                info!(
                    daemon_action = daemon_action.as_str(),
                    runtime_endpoint = %self.placement.endpoint().path().display(),
                    session_id = %session.session_id(),
                    available_capabilities = %session.available_capabilities(),
                    lease_duration_ms = session.lease().duration().as_millis(),
                    "Runtime session acquired"
                );
                Ok(SessionAttempt::Opened(Box::new(Session::new(
                    client.clone(),
                    session.available_capabilities().clone(),
                    {
                        #[cfg(not(test))]
                        {
                            crate::SessionHandle::daemon(
                                client,
                                session.session_id().clone(),
                                session.lease(),
                            )
                        }
                        #[cfg(test)]
                        {
                            crate::SessionHandle::daemon(
                                client,
                                session.session_id().clone(),
                                session.lease(),
                                config.session().renewal_observer(),
                            )
                        }
                    },
                ))))
            }
            Err(error) => match SessionOpenFailure::from_error(self.placement, error) {
                SessionOpenFailure::MissingEndpoint(incompatible) => {
                    Ok(SessionAttempt::Incompatible(incompatible))
                }
                SessionOpenFailure::Rejected(rejected) => rejected.fail(),
                SessionOpenFailure::Client(error) => Err(error.into()),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct Missing {
    placement: PlacementSpec,
}

impl Missing {
    fn placement(&self) -> &PlacementSpec {
        &self.placement
    }

    async fn start(self, config: &RuntimeConfig, _held: &Held) -> Result<Connected> {
        let mut daemon_handle =
            DaemonLauncher::new(config.daemon(), self.placement.clone()).launch()?;
        let connected = wait_connected(
            self.placement,
            config.session().acquire_timeout(),
            EndpointReadyWaitDaemon::Launched(&mut daemon_handle),
        )
        .await?;
        daemon_handle.reap_in_background();

        Ok(connected)
    }
}

#[derive(Debug, Clone)]
struct Stale {
    placement: PlacementSpec,
}

impl Stale {
    fn placement(&self) -> &PlacementSpec {
        &self.placement
    }

    fn cleanup(self, _held: &Held) -> Result<Missing> {
        #[cfg(unix)]
        {
            let endpoint_path = self.placement.endpoint().path();
            match std::fs::symlink_metadata(endpoint_path) {
                Ok(metadata) if metadata.file_type().is_socket() => {
                    std::fs::remove_file(endpoint_path)?;
                    info!(
                        runtime_endpoint = %endpoint_path.display(),
                        "Removed stale runtime endpoint"
                    );
                }
                Ok(_) => {
                    return Err(Error::NonSocketEndpoint {
                        path: endpoint_path.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }

            Ok(Missing {
                placement: self.placement,
            })
        }

        #[cfg(not(unix))]
        {
            let _ = self;
            let _ = _held;

            Err(Error::UnsupportedTransport {
                context: "recovering stale runtime endpoint",
            })
        }
    }
}

#[derive(Debug, Clone)]
struct Unavailable {
    placement: PlacementSpec,
}

impl Unavailable {
    fn fail<T>(self, context: &'static str) -> Result<T> {
        debug!(
            runtime_endpoint = %self.placement.endpoint().path().display(),
            context,
            "Runtime endpoint is unavailable"
        );
        Err(Error::EndpointUnavailable {
            context,
            endpoint: self.placement.endpoint().path().to_path_buf(),
        })
    }
}

#[cfg(not(unix))]
#[derive(Debug, Clone)]
struct UnsupportedEndpoint {
    placement: PlacementSpec,
}

#[cfg(not(unix))]
impl UnsupportedEndpoint {
    fn fail<T>(self, context: &'static str) -> Result<T> {
        debug!(
            runtime_endpoint = %self.placement.endpoint().path().display(),
            context,
            "Runtime endpoint transport is unsupported"
        );

        Err(Error::UnsupportedTransport { context })
    }
}

enum SessionAttempt {
    Opened(Box<Session>),
    Incompatible(Incompatible),
}

impl std::fmt::Debug for SessionAttempt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Opened(_) => formatter.debug_tuple("Opened").field(&"<session>").finish(),
            Self::Incompatible(incompatible) => formatter
                .debug_tuple("Incompatible")
                .field(incompatible)
                .finish(),
        }
    }
}

#[derive(Debug)]
enum SessionOpenFailure {
    MissingEndpoint(Incompatible),
    Rejected(SessionOpenRejection),
    Client(synd_client::SyndApiError),
}

impl SessionOpenFailure {
    fn from_error(placement: PlacementSpec, error: synd_client::SyndApiError) -> Self {
        if session_endpoint_missing(&error) {
            debug!(
                runtime_endpoint = %placement.endpoint().path().display(),
                "Daemon does not expose session open endpoint"
            );
            return Self::MissingEndpoint(Incompatible { placement });
        }

        match error {
            synd_client::SyndApiError::OpenSession(response) => {
                Self::Rejected(SessionOpenRejection {
                    placement,
                    response,
                })
            }
            error => Self::Client(error),
        }
    }
}

#[derive(Debug, Clone)]
struct SessionOpenRejection {
    placement: PlacementSpec,
    response: OpenSessionErrorResponse,
}

impl SessionOpenRejection {
    fn fail<T>(self) -> Result<T> {
        match self.response.code() {
            OpenSessionErrorCode::MissingCapabilities => {
                let missing_capabilities = self.response.missing_capabilities().clone();
                debug!(
                    runtime_endpoint = %self.placement.endpoint().path().display(),
                    missing_capabilities = %missing_capabilities,
                    "Daemon rejected session open because required capabilities are missing"
                );
                Err(Error::MissingSessionCapabilities {
                    endpoint: self.placement.endpoint().path().to_path_buf(),
                    missing_capabilities,
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Incompatible {
    placement: PlacementSpec,
}

impl Incompatible {
    fn placement(&self) -> &PlacementSpec {
        &self.placement
    }

    async fn stop(self, config: &RuntimeConfig, _held: &Held) -> Result<Stopped> {
        let suggestion = daemon_stop_suggestion(&self.placement);
        let client = daemon_client(config, &self.placement)?;
        client.shutdown_daemon().await.map_err(|source| {
            Error::IncompatibleRuntimeDaemonShutdown {
                endpoint: self.placement.endpoint().path().to_path_buf(),
                suggestion: suggestion.clone(),
                source: Box::new(source),
            }
        })?;

        match wait_stopped(self.placement.clone(), config.session().acquire_timeout()).await {
            Ok(stopped) => Ok(stopped),
            Err(Error::EndpointStopTimeout { .. }) => {
                Err(Error::IncompatibleRuntimeDaemonStopTimeout {
                    endpoint: self.placement.endpoint().path().to_path_buf(),
                    suggestion,
                })
            }
            Err(error) => Err(error),
        }
    }

    fn fail<T>(self) -> Result<T> {
        let suggestion = daemon_stop_suggestion(&self.placement);
        debug!(
            runtime_endpoint = %self.placement.endpoint().path().display(),
            suggestion = %suggestion,
            "Runtime daemon is incompatible"
        );
        Err(Error::IncompatibleRuntimeDaemon {
            endpoint: self.placement.endpoint().path().to_path_buf(),
            suggestion,
        })
    }
}

#[derive(Debug)]
enum Stopped {
    Missing(Missing),
    Stale(Stale),
}

enum Startup {
    Held(Held),
    Busy(Busy),
    #[cfg(not(unix))]
    Unsupported(UnsupportedStartup),
}

impl Startup {
    fn acquire(placement: &PlacementSpec) -> Result<Self> {
        match StartupLockAcquirer::new(placement.startup_lock_path()).try_acquire()? {
            StartupLockAcquisition::Acquired(lock) => Ok(Self::Held(Held { _lock: lock })),
            StartupLockAcquisition::AlreadyHeld => Ok(Self::Busy(Busy {
                placement: placement.clone(),
            })),
            #[cfg(not(unix))]
            StartupLockAcquisition::UnsupportedTransport => {
                Ok(Self::Unsupported(UnsupportedStartup {}))
            }
        }
    }
}

#[derive(Debug)]
struct Held {
    _lock: StartupLock,
}

#[derive(Debug, Clone)]
struct Busy {
    placement: PlacementSpec,
}

impl Busy {
    async fn wait(self, config: &RuntimeConfig) -> Result<Connected> {
        wait_connected(
            self.placement,
            config.session().acquire_timeout(),
            EndpointReadyWaitDaemon::External,
        )
        .await
    }
}

#[cfg(not(unix))]
struct UnsupportedStartup {}

#[cfg(not(unix))]
impl UnsupportedStartup {
    fn fail<T>(self, context: &'static str) -> Result<T> {
        let _ = self;
        debug!(context, "Runtime startup lock transport is unsupported");

        Err(Error::UnsupportedTransport { context })
    }
}

#[cfg(unix)]
fn daemon_client(config: &RuntimeConfig, placement: &PlacementSpec) -> Result<synd_client::Client> {
    Ok(synd_client::Client::new_unix(
        placement.endpoint().path(),
        synd_client::ClientOptions::new(
            config.client().request_timeout(),
            config.client().user_agent(),
        ),
    )?)
}

#[cfg(not(unix))]
fn daemon_client(config: &RuntimeConfig, placement: &PlacementSpec) -> Result<synd_client::Client> {
    let _ = (config, placement);

    Err(Error::UnsupportedTransport {
        context: "runtime endpoint session",
    })
}

async fn wait_connected(
    placement: PlacementSpec,
    timeout: Duration,
    mut daemon: EndpointReadyWaitDaemon<'_>,
) -> Result<Connected> {
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(exited) = daemon.try_exited()? {
            debug!(
                status = %exited.status,
                daemon_launch = ?exited.launch,
                "Daemon exited before endpoint became ready"
            );
            return Err(Error::DaemonExitedBeforeReady {
                status: exited.status,
                launch: exited.launch,
            });
        }

        let endpoint = Endpoint::probe(placement.clone()).await;
        if let Endpoint::Connected(connected) = endpoint {
            return Ok(connected);
        }

        let now = Instant::now();
        if now >= deadline {
            return Err(daemon.timeout_error(&placement));
        }

        debug!(
            runtime_endpoint = %placement.endpoint().path().display(),
            runtime_endpoint_state = endpoint.name(),
            "Waiting for daemon endpoint"
        );
        sleep(ENDPOINT_WAIT_INTERVAL.min(deadline - now)).await;
    }
}

enum EndpointReadyWaitDaemon<'a> {
    Launched(&'a mut DaemonHandle),
    External,
}

impl EndpointReadyWaitDaemon<'_> {
    fn try_exited(&mut self) -> Result<Option<ExitedDaemon>> {
        match self {
            Self::Launched(handle) => {
                let status = handle.try_wait()?;
                Ok(status.map(|status| ExitedDaemon {
                    status,
                    launch: handle.launch().clone(),
                }))
            }
            Self::External => Ok(None),
        }
    }

    fn timeout_error(&self, placement: &PlacementSpec) -> Error {
        let endpoint = placement.endpoint().path().to_path_buf();
        match self {
            Self::Launched(handle) => Error::DaemonEndpointReadyTimeout {
                endpoint,
                launch: handle.launch().clone(),
            },
            Self::External => Error::EndpointReadyTimeout { endpoint },
        }
    }
}

struct ExitedDaemon {
    status: ExitStatus,
    launch: crate::DaemonLaunchInfo,
}

async fn wait_stopped(placement: PlacementSpec, timeout: Duration) -> Result<Stopped> {
    let deadline = Instant::now() + timeout;

    loop {
        match Endpoint::probe(placement.clone()).await {
            Endpoint::Missing(missing) => return Ok(Stopped::Missing(missing)),
            Endpoint::Stale(stale) => return Ok(Stopped::Stale(stale)),
            Endpoint::Connected(_) => {}
            Endpoint::Unavailable(unavailable) => return unavailable.fail("runtime endpoint"),
            #[cfg(not(unix))]
            Endpoint::Unsupported(unsupported) => return unsupported.fail("runtime endpoint"),
        }

        let now = Instant::now();
        if now >= deadline {
            return Err(Error::EndpointStopTimeout {
                endpoint: placement.endpoint().path().to_path_buf(),
            });
        }

        sleep(ENDPOINT_WAIT_INTERVAL.min(deadline - now)).await;
    }
}

fn session_endpoint_missing(error: &synd_client::SyndApiError) -> bool {
    matches!(
        error,
        synd_client::SyndApiError::HttpStatus {
            status,
            url: Some(url),
        } if status.as_u16() == 404 && url.path() == "/session/open"
    )
}

fn daemon_stop_suggestion(placement: &PlacementSpec) -> String {
    format!(
        "run `synd --sqlite-db {} daemon shutdown` and retry",
        shell_quote(placement.instance().canonical_database_path())
    )
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    if value.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '+' | '=' | ':' | '@')
    }) {
        return value.into_owned();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        path::{Path, PathBuf},
        time::Duration,
    };

    #[cfg(unix)]
    use std::os::unix::net::UnixListener;

    use crate::{
        RuntimeConfig, RuntimeDatabase,
        connection::RuntimeEndpointConnectionStatus,
        instance::RuntimeInstance,
        placement::{PlacementRoot, PlacementSpec},
    };

    use super::{
        Connected, DaemonAction, Endpoint, Held, Incompatible, SessionAttempt, Stale, Stopped,
    };

    mod endpoint_state {
        use super::*;

        #[test]
        fn from_connection() {
            let cases = [
                (RuntimeEndpointConnectionStatus::Connected, "connected"),
                (RuntimeEndpointConnectionStatus::Missing, "missing"),
                (RuntimeEndpointConnectionStatus::Stale, "stale"),
                (RuntimeEndpointConnectionStatus::Unavailable, "unavailable"),
            ];

            for (connection, expected) in cases {
                let endpoint = Endpoint::from_connection(placement(), connection);

                assert_eq!(endpoint.name(), expected);
            }

            #[cfg(not(unix))]
            {
                let endpoint = Endpoint::from_connection(
                    placement(),
                    RuntimeEndpointConnectionStatus::UnsupportedTransport,
                );

                assert_eq!(endpoint.name(), "unsupported");
            }
        }
    }

    mod daemon_stop_suggestion {
        use super::*;

        #[test]
        fn uses_runtime_database() {
            let placement = placement();
            let expected_db = placement.instance().canonical_database_path().display();

            assert_eq!(
                super::super::daemon_stop_suggestion(&placement),
                format!("run `synd --sqlite-db {expected_db} daemon shutdown` and retry")
            );
        }
    }

    mod shell_quote {
        use super::*;

        #[test]
        fn quotes_spaces() {
            assert_eq!(
                super::super::shell_quote(Path::new("/tmp/synd db/synd's.db")),
                "'/tmp/synd db/synd'\\''s.db'"
            );
        }
    }

    mod session_endpoint_missing {
        #[test]
        fn detects_open_404() {
            let error = synd_client::SyndApiError::HttpStatus {
                status: 404.try_into().unwrap(),
                url: Some(url::Url::parse("http://localhost/session/open").unwrap()),
            };

            assert!(super::super::session_endpoint_missing(&error));
        }
    }

    #[cfg(unix)]
    mod connected_open {
        use super::*;
        use core::assert_matches;

        #[tokio::test]
        async fn reports_incompatible() {
            let placement = placement();
            let server = spawn_old_daemon(placement.endpoint().path().to_path_buf());
            let config = runtime_config_for(&placement);

            let attempt = Connected {
                placement: placement.clone(),
            }
            .open(&config, DaemonAction::Existing)
            .await
            .unwrap();

            assert_matches!(attempt, SessionAttempt::Incompatible(_));
            server.join().unwrap();
        }
    }

    #[cfg(unix)]
    mod incompatible_stop {
        use super::*;
        use core::assert_matches;

        #[tokio::test]
        async fn returns_stale() {
            let held = held();
            let placement = placement();
            let server = spawn_old_daemon(placement.endpoint().path().to_path_buf());
            let config = runtime_config_for(&placement);

            let stopped = Incompatible {
                placement: placement.clone(),
            }
            .stop(&config, &held)
            .await
            .unwrap();

            assert_matches!(stopped, Stopped::Stale(_));
            server.join().unwrap();
        }
    }

    #[cfg(unix)]
    mod stale_endpoint_recovery {
        use super::*;

        #[test]
        fn removes_socket_file() {
            let held = held();
            let placement = placement();
            let endpoint = placement.endpoint().path().to_path_buf();
            std::fs::create_dir_all(endpoint.parent().unwrap()).unwrap();
            let listener = UnixListener::bind(&endpoint).unwrap();
            drop(listener);

            let recovered = Stale { placement }.cleanup(&held).unwrap();

            assert_eq!(recovered.placement.endpoint().path(), endpoint.as_path());
            assert!(!endpoint.exists());
        }

        #[test]
        fn refuses_non_socket_file() {
            let held = held();
            let placement = placement();
            let endpoint = placement.endpoint().path().to_path_buf();
            std::fs::create_dir_all(endpoint.parent().unwrap()).unwrap();
            std::fs::write(&endpoint, "").unwrap();

            let error = Stale { placement }.cleanup(&held).unwrap_err();

            assert!(error.to_string().contains("non-socket runtime endpoint"));
            assert!(endpoint.exists());
        }
    }

    fn placement() -> PlacementSpec {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();
        PlacementSpec::from_instance(PlacementRoot::from(tmp.path().join("runtime")), instance)
    }

    fn runtime_config_for(placement: &PlacementSpec) -> RuntimeConfig {
        RuntimeConfig::new(RuntimeDatabase::sqlite(
            placement.instance().canonical_database_path(),
        ))
        .with_api_timeout(Duration::from_secs(2), "synd-runtime-test")
        .with_session_timeout(Duration::from_secs(2))
    }

    #[cfg(unix)]
    fn spawn_old_daemon(endpoint: PathBuf) -> std::thread::JoinHandle<()> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let bind_endpoint = endpoint.clone();
        let handle = std::thread::spawn(move || {
            std::fs::create_dir_all(endpoint.parent().unwrap()).unwrap();
            let listener = match UnixListener::bind(&endpoint) {
                Ok(listener) => listener,
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            };
            ready_tx.send(Ok(())).unwrap();
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let len = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..len]);
            let response = if request.starts_with("POST /daemon/shutdown ") {
                "HTTP/1.1 204 No Content\r\ncontent-length: 0\r\n\r\n"
            } else if request.starts_with("POST /session/open ") {
                "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n"
            } else {
                "HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n"
            };
            stream.write_all(response.as_bytes()).unwrap();
        });

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => handle,
            Ok(Err(error)) => panic!(
                "failed to bind old daemon endpoint {}: {error}",
                bind_endpoint.display()
            ),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => panic!(
                "timed out waiting for old daemon endpoint {} to bind",
                bind_endpoint.display()
            ),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => panic!(
                "old daemon thread exited before binding endpoint {}",
                bind_endpoint.display()
            ),
        }
    }

    #[cfg(unix)]
    fn held() -> Held {
        let tmp = tempfile::tempdir().unwrap();
        let placement = placement();
        let lock_path = crate::startup::StartupLockPath::from_instance_id(
            tmp.path(),
            placement.instance().id(),
        );
        match crate::startup::StartupLockAcquirer::new(&lock_path)
            .try_acquire()
            .unwrap()
        {
            crate::startup::StartupLockAcquisition::Acquired(lock) => Held { _lock: lock },
            crate::startup::StartupLockAcquisition::AlreadyHeld => {
                panic!("startup lock should be available")
            }
            #[cfg(not(unix))]
            crate::startup::StartupLockAcquisition::UnsupportedTransport => {
                panic!("startup lock should be supported")
            }
        }
    }
}
