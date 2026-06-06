use std::{
    fs, io,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt as _;

use clap::Args;
use serde::Serialize;
use synd_persistence::sqlite::SqliteDatabase;
use synd_runtime::{DaemonState, Runtime, RuntimeConfig, RuntimeDatabase, RuntimePlacementSummary};
use tracing::error;

use crate::{cli::OutputFormat, config::ConfigResolver};

const DOCTOR_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Diagnose local environment
#[derive(Args, Debug)]
pub struct DoctorCommand {
    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,
}

impl DoctorCommand {
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        match self.diagnose(config).await {
            Ok(has_failures) if has_failures => ExitCode::from(1),
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                error!("{err:?}");
                ExitCode::from(1)
            }
        }
    }

    async fn diagnose(self, config: ConfigResolver) -> anyhow::Result<bool> {
        let output = DoctorOutput::from_config(&config).await;
        let has_failures = output.has_failures();

        match self.output {
            OutputFormat::Human => output.print(io::stdout())?,
            OutputFormat::Json => {
                let mut stdout = io::stdout();
                serde_json::to_writer_pretty(&mut stdout, &output)?;
                writeln!(stdout)?;
            }
        }

        Ok(has_failures)
    }
}

#[derive(Debug, Serialize)]
struct DoctorOutput {
    checks: Vec<DoctorCheck>,
}

impl DoctorOutput {
    async fn from_config(config: &ConfigResolver) -> Self {
        let sqlite_db = config.sqlite_db();

        let mut checks = vec![
            existing_file(
                "config_file",
                config.config_file(),
                MissingSeverity::Warn,
                "config file does not exist",
            ),
            existing_directory("cache_dir", config.cache_dir(), MissingSeverity::Warn),
            existing_parent("log_parent", &config.log_file(), MissingSeverity::Warn),
            existing_parent("sqlite_parent", &sqlite_db, MissingSeverity::Fail),
            sqlite_database(&sqlite_db).await,
        ];
        checks.extend(RuntimeDoctor::from_config(config).diagnose().await);

        Self { checks }
    }

    fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Fail)
    }

    fn print(&self, mut writer: impl io::Write) -> io::Result<()> {
        for check in &self.checks {
            match &check.path {
                Some(path) => writeln!(
                    writer,
                    "{:>4} {:<16} {} ({})",
                    check.status.label(),
                    check.name,
                    check.message,
                    path.display()
                )?,
                None => writeln!(
                    writer,
                    "{:>4} {:<16} {}",
                    check.status.label(),
                    check.name,
                    check.message
                )?,
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: &'static str,
    status: DoctorStatus,
    path: Option<PathBuf>,
    message: String,
}

impl DoctorCheck {
    fn pass(name: &'static str, path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            name,
            status: DoctorStatus::Pass,
            path: Some(path.into()),
            message: message.into(),
        }
    }

    fn warn(name: &'static str, path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            name,
            status: DoctorStatus::Warn,
            path: Some(path.into()),
            message: message.into(),
        }
    }

    fn fail(name: &'static str, path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            name,
            status: DoctorStatus::Fail,
            path: Some(path.into()),
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl DoctorStatus {
    const fn label(self) -> &'static str {
        match self {
            DoctorStatus::Pass => "PASS",
            DoctorStatus::Warn => "WARN",
            DoctorStatus::Fail => "FAIL",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum MissingSeverity {
    Warn,
    Fail,
}

fn existing_file(
    name: &'static str,
    path: PathBuf,
    missing_severity: MissingSeverity,
    missing_message: &'static str,
) -> DoctorCheck {
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => DoctorCheck::pass(name, path, "file exists"),
        Ok(_) => DoctorCheck::fail(name, path, "path exists but is not a file"),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            missing_check(name, path, missing_severity, missing_message)
        }
        Err(err) => DoctorCheck::fail(name, path, format!("failed to inspect path: {err}")),
    }
}

fn existing_directory(
    name: &'static str,
    path: PathBuf,
    missing_severity: MissingSeverity,
) -> DoctorCheck {
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => DoctorCheck::pass(name, path, "directory exists"),
        Ok(_) => DoctorCheck::fail(name, path, "path exists but is not a directory"),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            missing_check(name, path, missing_severity, "directory does not exist")
        }
        Err(err) => DoctorCheck::fail(name, path, format!("failed to inspect path: {err}")),
    }
}

fn existing_parent(
    name: &'static str,
    path: &Path,
    missing_severity: MissingSeverity,
) -> DoctorCheck {
    let parent = path.parent().unwrap_or_else(|| Path::new(".")).to_owned();
    existing_directory(name, parent, missing_severity)
}

struct RuntimeDoctor {
    config: RuntimeDoctorConfig,
}

impl RuntimeDoctor {
    fn from_config(config: &ConfigResolver) -> Self {
        Self {
            config: RuntimeDoctorConfig::from(config),
        }
    }

    async fn diagnose(self) -> Vec<DoctorCheck> {
        let runtime = match Runtime::try_new(self.config.runtime_config()) {
            Ok(runtime) => runtime,
            Err(error) => {
                return vec![DoctorCheck::fail(
                    "runtime_placement",
                    self.config.sqlite_db,
                    format!("failed to resolve runtime placement: {error}"),
                )];
            }
        };

        RuntimeDoctorReport::inspect(runtime, self.config.log_file)
            .await
            .into_checks()
    }
}

struct RuntimeDoctorConfig {
    sqlite_db: PathBuf,
    api_timeout: Duration,
    runtime_root: Option<PathBuf>,
    log_file: PathBuf,
}

impl RuntimeDoctorConfig {
    fn runtime_config(&self) -> RuntimeConfig {
        let config = RuntimeConfig::new(RuntimeDatabase::sqlite(self.sqlite_db.clone()))
            .with_api_timeout(self.api_timeout, DOCTOR_USER_AGENT);

        match &self.runtime_root {
            Some(root) => config.with_runtime_root(root),
            None => config,
        }
    }
}

impl From<&ConfigResolver> for RuntimeDoctorConfig {
    fn from(config: &ConfigResolver) -> Self {
        Self {
            sqlite_db: config.sqlite_db(),
            api_timeout: config.api_timeout(),
            runtime_root: config.daemon_runtime_root(),
            log_file: config.log_file(),
        }
    }
}

struct RuntimeDoctorReport {
    placement: RuntimePlacementSummary,
    daemon_status: synd_runtime::Result<synd_runtime::DaemonStatus>,
    log_file: PathBuf,
}

impl RuntimeDoctorReport {
    async fn inspect(runtime: Runtime, log_file: PathBuf) -> Self {
        let placement = runtime.placement_summary();
        let daemon_status = runtime.daemon().inspect().await;

        Self {
            placement,
            daemon_status,
            log_file,
        }
    }

    fn into_checks(self) -> Vec<DoctorCheck> {
        let daemon_state = self
            .daemon_status
            .as_ref()
            .ok()
            .map(synd_runtime::DaemonStatus::state);
        let mut checks = vec![
            self.runtime_instance_check(),
            self.runtime_root_check(),
            self.runtime_endpoint_check(daemon_state),
            self.startup_lock_check(),
            self.daemon_log_check(),
        ];
        checks.push(self.daemon_status_check());

        checks
    }

    fn runtime_instance_check(&self) -> DoctorCheck {
        DoctorCheck::pass(
            "runtime_instance",
            self.placement.database().to_path_buf(),
            format!("instance {}", self.placement.runtime_instance_id()),
        )
    }

    fn runtime_root_check(&self) -> DoctorCheck {
        existing_directory(
            "runtime_root",
            self.placement.runtime_root().to_path_buf(),
            MissingSeverity::Warn,
        )
    }

    fn runtime_endpoint_check(&self, daemon_state: Option<DaemonState>) -> DoctorCheck {
        if daemon_state == Some(DaemonState::Running) {
            return DoctorCheck::pass(
                "runtime_endpoint",
                self.placement.endpoint().to_path_buf(),
                "endpoint accepts connections",
            );
        }

        self.runtime_endpoint_file_check()
    }

    #[cfg(unix)]
    fn runtime_endpoint_file_check(&self) -> DoctorCheck {
        let path = self.placement.endpoint();
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_socket() => {
                DoctorCheck::warn("runtime_endpoint", path, "stale daemon socket exists")
            }
            Ok(_) => DoctorCheck::fail("runtime_endpoint", path, "path exists but is not a socket"),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                DoctorCheck::pass("runtime_endpoint", path, "daemon endpoint does not exist")
            }
            Err(err) => DoctorCheck::fail(
                "runtime_endpoint",
                path,
                format!("failed to inspect endpoint: {err}"),
            ),
        }
    }

    #[cfg(not(unix))]
    fn runtime_endpoint_file_check(&self) -> DoctorCheck {
        DoctorCheck::warn(
            "runtime_endpoint",
            self.placement.endpoint(),
            "runtime endpoint diagnostics are not implemented for this platform",
        )
    }

    fn startup_lock_check(&self) -> DoctorCheck {
        let path = self.placement.startup_lock();
        match fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => {
                DoctorCheck::pass("startup_lock", path, "startup lock file exists")
            }
            Ok(_) => DoctorCheck::fail("startup_lock", path, "path exists but is not a file"),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                DoctorCheck::pass("startup_lock", path, "startup lock file does not exist")
            }
            Err(err) => DoctorCheck::fail(
                "startup_lock",
                path,
                format!("failed to inspect startup lock: {err}"),
            ),
        }
    }

    fn daemon_log_check(&self) -> DoctorCheck {
        match fs::metadata(&self.log_file) {
            Ok(metadata) if metadata.is_file() => {
                DoctorCheck::pass("daemon_log", &self.log_file, "daemon log file exists")
            }
            Ok(_) => DoctorCheck::fail(
                "daemon_log",
                &self.log_file,
                "path exists but is not a file",
            ),
            Err(err) if err.kind() == io::ErrorKind::NotFound => DoctorCheck::warn(
                "daemon_log",
                &self.log_file,
                "daemon log does not exist yet",
            ),
            Err(err) => DoctorCheck::fail(
                "daemon_log",
                &self.log_file,
                format!("failed to inspect daemon log: {err}"),
            ),
        }
    }

    fn daemon_status_check(self) -> DoctorCheck {
        match self.daemon_status {
            Ok(status) => {
                let message = match status.state() {
                    DaemonState::Running => match status.sessions() {
                        Some(sessions) => {
                            format!(
                                "daemon running, {} active sessions",
                                sessions.active_sessions()
                            )
                        }
                        None => "daemon running".to_owned(),
                    },
                    DaemonState::NotRunning => "daemon not running".to_owned(),
                };

                DoctorCheck::pass(
                    "daemon_status",
                    status.placement().endpoint().to_path_buf(),
                    message,
                )
            }
            Err(error) => DoctorCheck::fail(
                "daemon_status",
                self.placement.endpoint().to_path_buf(),
                format!("failed to inspect daemon: {error}"),
            ),
        }
    }
}

async fn sqlite_database(path: &Path) -> DoctorCheck {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => match SqliteDatabase::open(path).await {
            Ok(_) => DoctorCheck::pass("sqlite_db", path, "database opens"),
            Err(err) => {
                DoctorCheck::fail("sqlite_db", path, format!("failed to open database: {err}"))
            }
        },
        Ok(_) => DoctorCheck::fail("sqlite_db", path, "path exists but is not a file"),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            DoctorCheck::warn("sqlite_db", path, "database does not exist yet")
        }
        Err(err) => DoctorCheck::fail("sqlite_db", path, format!("failed to inspect path: {err}")),
    }
}

fn missing_check(
    name: &'static str,
    path: PathBuf,
    severity: MissingSeverity,
    message: impl Into<String>,
) -> DoctorCheck {
    match severity {
        MissingSeverity::Warn => DoctorCheck::warn(name, path, message),
        MissingSeverity::Fail => DoctorCheck::fail(name, path, message),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    mod runtime_endpoint {
        use super::*;

        #[cfg(unix)]
        #[test]
        fn reports_missing() {
            let (_tmp, report) = report();

            let check = report.runtime_endpoint_file_check();

            assert_eq!(check.status, DoctorStatus::Pass);
            assert_eq!(check.message, "daemon endpoint does not exist");
        }

        #[cfg(unix)]
        #[test]
        fn refuses_non_socket() {
            let (_tmp, report) = report();
            let endpoint = report.placement.endpoint();
            std::fs::create_dir_all(endpoint.parent().unwrap()).unwrap();
            std::fs::write(endpoint, "").unwrap();

            let check = report.runtime_endpoint_file_check();

            assert_eq!(check.status, DoctorStatus::Fail);
            assert_eq!(check.message, "path exists but is not a socket");
        }
    }

    mod startup_lock {
        use super::*;

        #[test]
        fn reports_missing() {
            let (_tmp, report) = report();

            let check = report.startup_lock_check();

            assert_eq!(check.status, DoctorStatus::Pass);
            assert_eq!(check.message, "startup lock file does not exist");
        }
    }

    mod daemon_log {
        use super::*;

        #[test]
        fn reports_missing() {
            let (_tmp, report) = report();

            let check = report.daemon_log_check();

            assert_eq!(check.status, DoctorStatus::Warn);
            assert_eq!(check.message, "daemon log does not exist yet");
        }
    }

    fn report() -> (TempDir, RuntimeDoctorReport) {
        let tmp = tempfile::tempdir().unwrap();
        let runtime_root = tmp.path().join("runtime");
        let database = tmp.path().join("synd.db");
        let config =
            RuntimeConfig::new(RuntimeDatabase::sqlite(database)).with_runtime_root(&runtime_root);
        let runtime = Runtime::try_new(config).unwrap();
        let placement = runtime.placement_summary();
        let endpoint = placement.endpoint().to_path_buf();
        let report = RuntimeDoctorReport {
            placement,
            daemon_status: Err(synd_runtime::Error::EndpointUnavailable {
                context: "test",
                endpoint,
            }),
            log_file: tmp.path().join("synd.log"),
        };

        (tmp, report)
    }
}
