use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};

use synd_api::session::DaemonSessionConfig;
use synd_support::{dirs::SyndicationdDirs, time::humantime::HumanDuration};
use tracing::{debug, warn};

use crate::{Result, placement::RuntimePlacement};

/// Configuration for starting a daemon process for a runtime instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonLaunchConfig {
    executable: DaemonExecutable,
    log: DaemonLaunchLog,
    session: DaemonSessionConfig,
}

impl DaemonLaunchConfig {
    pub fn new(executable: DaemonExecutable, log: DaemonLaunchLog) -> Self {
        Self {
            executable,
            log,
            session: DaemonSessionConfig::default(),
        }
    }

    pub fn executable(&self) -> &DaemonExecutable {
        &self.executable
    }

    pub fn log(&self) -> &DaemonLaunchLog {
        &self.log
    }

    pub fn session(&self) -> DaemonSessionConfig {
        self.session
    }

    #[must_use]
    pub fn with_log(mut self, log: DaemonLaunchLog) -> Self {
        self.log = log;
        self
    }

    #[must_use]
    pub fn with_session(mut self, session: DaemonSessionConfig) -> Self {
        self.session = session;
        self
    }

    #[must_use]
    pub fn with_session_lease_duration(mut self, lease_duration: Duration) -> Self {
        self.session = self.session.with_lease_duration(lease_duration);
        self
    }

    #[must_use]
    pub fn with_session_idle_shutdown_grace(mut self, idle_shutdown_grace: Duration) -> Self {
        self.session = self.session.with_idle_shutdown_grace(idle_shutdown_grace);
        self
    }
}

impl Default for DaemonLaunchConfig {
    fn default() -> Self {
        Self::new(
            DaemonExecutable::current(),
            DaemonLaunchLog::file(SyndicationdDirs::current().log_file()),
        )
    }
}

/// Executable used to start a daemon process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonExecutable {
    kind: DaemonExecutableKind,
}

impl DaemonExecutable {
    pub fn current() -> Self {
        Self {
            kind: DaemonExecutableKind::Current,
        }
    }

    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self {
            kind: DaemonExecutableKind::Path(path.into()),
        }
    }

    fn resolve(&self) -> Result<PathBuf> {
        Ok(match &self.kind {
            DaemonExecutableKind::Current => std::env::current_exe()?,
            DaemonExecutableKind::Path(path) => path.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonExecutableKind {
    Current,
    Path(PathBuf),
}

/// File target for daemon stdout and stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonLaunchLog {
    path: PathBuf,
}

impl DaemonLaunchLog {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn open(&self) -> Result<OpenedDaemonLaunchLog> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let stdout = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)?;
        let stderr = stdout.try_clone()?;

        Ok(OpenedDaemonLaunchLog { stdout, stderr })
    }
}

struct OpenedDaemonLaunchLog {
    stdout: File,
    stderr: File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedDaemonLaunchCommand {
    executable: PathBuf,
    arguments: DaemonServeArguments,
}

impl ResolvedDaemonLaunchCommand {
    fn daemon_serve(
        executable: PathBuf,
        database_path: &Path,
        session: DaemonSessionConfig,
    ) -> Self {
        Self {
            executable,
            arguments: DaemonServeArguments::new(database_path, session),
        }
    }
}

/// Argument vector for invoking `synd daemon serve`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonServeArguments {
    values: Vec<OsString>,
}

impl DaemonServeArguments {
    fn new(database_path: &Path, session: DaemonSessionConfig) -> Self {
        let mut values = vec![
            OsString::from("daemon"),
            OsString::from("serve"),
            OsString::from("--sqlite-db"),
            database_path.as_os_str().to_os_string(),
        ];
        values.extend(DaemonSessionServeArguments::from(session));

        Self { values }
    }

    fn as_slice(&self) -> &[OsString] {
        &self.values
    }
}

/// Session-related argument vector fragment for `synd daemon serve`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonSessionServeArguments {
    lease_duration: Duration,
    idle_shutdown_grace: Duration,
}

impl From<DaemonSessionConfig> for DaemonSessionServeArguments {
    fn from(config: DaemonSessionConfig) -> Self {
        Self {
            lease_duration: config.lease_policy().lease_duration(),
            idle_shutdown_grace: config.idle_shutdown_grace(),
        }
    }
}

impl IntoIterator for DaemonSessionServeArguments {
    type Item = OsString;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            OsString::from("--session-lease-duration"),
            OsString::from(String::from(HumanDuration::from(self.lease_duration))),
            OsString::from("--session-idle-shutdown-grace"),
            OsString::from(String::from(HumanDuration::from(self.idle_shutdown_grace))),
        ]
        .into_iter()
    }
}

/// Starts a daemon process for a resolved runtime placement.
pub(crate) struct DaemonLauncher<'a> {
    config: &'a DaemonLaunchConfig,
    placement: RuntimePlacement,
}

impl<'a> DaemonLauncher<'a> {
    pub(crate) fn new(config: &'a DaemonLaunchConfig, placement: RuntimePlacement) -> Self {
        Self { config, placement }
    }

    pub(crate) fn launch(self) -> Result<DaemonHandle> {
        let command = ResolvedDaemonLaunchCommand::daemon_serve(
            self.config.executable().resolve()?,
            self.placement.instance().canonical_database_path(),
            self.config.session(),
        );
        let log = self.config.log().open()?;
        debug!(
            daemon_executable = %command.executable.display(),
            daemon_arguments = ?command.arguments,
            daemon_log = %self.config.log().path().display(),
            "Launching daemon"
        );

        let child = Command::new(&command.executable)
            .args(command.arguments.as_slice())
            .stdin(Stdio::null())
            .stdout(Stdio::from(log.stdout))
            .stderr(Stdio::from(log.stderr))
            .spawn()?;

        Ok(DaemonHandle { child })
    }
}

/// Handle for a daemon process spawned by session acquisition.
pub(crate) struct DaemonHandle {
    child: Child,
}

impl DaemonHandle {
    pub(crate) fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        Ok(self.child.try_wait()?)
    }

    pub(crate) fn reap_in_background(mut self) {
        std::thread::spawn(move || match self.child.wait() {
            Ok(status) => {
                debug!(%status, "Daemon process exited");
            }
            Err(error) => {
                warn!(%error, "Failed to wait for daemon process");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::Path, time::Duration};

    use synd_api::session::{DaemonSessionConfig, DaemonSessionLeasePolicy};

    use super::{DaemonExecutable, DaemonLaunchLog};

    #[test]
    fn builds_daemon_serve_command_from_runtime_database_path() {
        let command = super::ResolvedDaemonLaunchCommand::daemon_serve(
            DaemonExecutable::path("/usr/bin/synd").resolve().unwrap(),
            Path::new("/tmp/synd.db"),
            DaemonSessionConfig::default(),
        );

        assert_eq!(command.executable, Path::new("/usr/bin/synd"));
        assert_eq!(
            command.arguments.as_slice(),
            [
                OsString::from("daemon"),
                OsString::from("serve"),
                OsString::from("--sqlite-db"),
                OsString::from("/tmp/synd.db"),
                OsString::from("--session-lease-duration"),
                OsString::from("30s"),
                OsString::from("--session-idle-shutdown-grace"),
                OsString::from("30s"),
            ]
        );
    }

    #[test]
    fn builds_daemon_serve_command_with_session_config() {
        let command = super::ResolvedDaemonLaunchCommand::daemon_serve(
            DaemonExecutable::path("/usr/bin/synd").resolve().unwrap(),
            Path::new("/tmp/synd.db"),
            DaemonSessionConfig::new(
                DaemonSessionLeasePolicy::new(Duration::from_mins(1), Duration::from_secs(5)),
                Duration::from_mins(2),
            ),
        );

        assert_eq!(
            command.arguments.as_slice(),
            [
                OsString::from("daemon"),
                OsString::from("serve"),
                OsString::from("--sqlite-db"),
                OsString::from("/tmp/synd.db"),
                OsString::from("--session-lease-duration"),
                OsString::from("1m"),
                OsString::from("--session-idle-shutdown-grace"),
                OsString::from("2m"),
            ]
        );
    }

    #[test]
    fn launch_log_creates_parent_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let log = DaemonLaunchLog::file(tmp.path().join("nested").join("daemon.log"));

        let _opened = log.open().unwrap();

        assert!(log.path().exists());
    }
}
