use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
};

use synd_support::dirs::SyndicationdDirs;
use tracing::{debug, warn};

use crate::{Result, placement::RuntimePlacement};

/// Configuration for starting a daemon process for a runtime instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonLaunchConfig {
    command: DaemonLaunchCommand,
    log: DaemonLaunchLog,
}

impl DaemonLaunchConfig {
    pub fn new(command: DaemonLaunchCommand, log: DaemonLaunchLog) -> Self {
        Self { command, log }
    }

    pub fn command(&self) -> &DaemonLaunchCommand {
        &self.command
    }

    pub fn log(&self) -> &DaemonLaunchLog {
        &self.log
    }
}

impl Default for DaemonLaunchConfig {
    fn default() -> Self {
        Self::new(
            DaemonLaunchCommand::current_executable(),
            DaemonLaunchLog::file(SyndicationdDirs::current().log_file()),
        )
    }
}

/// Command template used to start a daemon process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonLaunchCommand {
    executable: DaemonLaunchExecutable,
}

impl DaemonLaunchCommand {
    pub fn current_executable() -> Self {
        Self {
            executable: DaemonLaunchExecutable::CurrentExecutable,
        }
    }

    pub fn executable(path: impl Into<PathBuf>) -> Self {
        Self {
            executable: DaemonLaunchExecutable::Path(path.into()),
        }
    }

    fn resolve_executable(&self) -> Result<PathBuf> {
        Ok(match &self.executable {
            DaemonLaunchExecutable::CurrentExecutable => std::env::current_exe()?,
            DaemonLaunchExecutable::Path(path) => path.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonLaunchExecutable {
    CurrentExecutable,
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
    arguments: Vec<OsString>,
}

impl ResolvedDaemonLaunchCommand {
    fn daemon_serve(executable: PathBuf, database_path: &Path) -> Self {
        Self {
            executable,
            arguments: vec![
                OsString::from("daemon"),
                OsString::from("serve"),
                OsString::from("--sqlite-db"),
                database_path.as_os_str().to_os_string(),
            ],
        }
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
            self.config.command().resolve_executable()?,
            self.placement.instance().canonical_database_path(),
        );
        let log = self.config.log().open()?;
        debug!(
            daemon_executable = %command.executable.display(),
            daemon_arguments = ?command.arguments,
            daemon_log = %self.config.log().path().display(),
            "Launching daemon"
        );

        let child = Command::new(&command.executable)
            .args(&command.arguments)
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
    use std::{ffi::OsString, path::Path};

    use super::{DaemonLaunchCommand, DaemonLaunchLog};

    #[test]
    fn builds_daemon_serve_command_from_runtime_database_path() {
        let command = super::ResolvedDaemonLaunchCommand::daemon_serve(
            DaemonLaunchCommand::executable("/usr/bin/synd")
                .resolve_executable()
                .unwrap(),
            Path::new("/tmp/synd.db"),
        );

        assert_eq!(command.executable, Path::new("/usr/bin/synd"));
        assert_eq!(
            command.arguments,
            [
                OsString::from("daemon"),
                OsString::from("serve"),
                OsString::from("--sqlite-db"),
                OsString::from("/tmp/synd.db"),
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
