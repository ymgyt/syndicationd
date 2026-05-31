use std::{
    fs, io,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Args;
use serde::Serialize;
use synd_persistence::sqlite::SqliteDatabase;
use tracing::error;

use crate::{cli::OutputFormat, config::ConfigResolver};

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

        let checks = vec![
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
