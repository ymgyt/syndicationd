use std::{io, path::PathBuf, process::ExitCode, time::Duration};

use clap::{Args, Subcommand};
use serde::Serialize;
use synd_runtime::{
    Daemon, DaemonConfig, DaemonIdleShutdownStatus, DaemonSessionStatus, DaemonState, DaemonStatus,
    RuntimeDatabase,
};
use synd_support::time::humantime::HumanDuration;

use crate::{
    cli::{OutputFormat, command::CommandFailure},
    config::{self, ConfigResolver},
    runtime::FeedRuntime,
};

/// Manage the runtime daemon
#[derive(Args, Debug)]
pub struct DaemonCommand {
    #[command(subcommand)]
    command: DaemonSubcommand,
}

#[derive(Subcommand, Debug)]
enum DaemonSubcommand {
    Serve(DaemonServeCommand),
    Status(DaemonStatusCommand),
    Shutdown(DaemonShutdownCommand),
}

impl DaemonCommand {
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        match self.command {
            DaemonSubcommand::Serve(serve) => serve.run(config).await,
            DaemonSubcommand::Status(status) => status.run(&config).await,
            DaemonSubcommand::Shutdown(shutdown) => shutdown.run(&config).await,
        }
    }
}

/// Serve one runtime instance as a daemon
#[derive(Args, Debug)]
struct DaemonServeCommand {
    /// `SQLite` database path served by this daemon
    #[arg(long = "sqlite-db")]
    sqlite_db: Option<PathBuf>,
    /// Session lease duration granted by this daemon
    #[arg(long, value_parser = config::parse::flag::parse_duration_opt, env = config::env::DAEMON_SESSION_LEASE_DURATION)]
    session_lease_duration: Option<Duration>,
    /// Grace period before this daemon shuts down after all sessions are gone
    #[arg(long, value_parser = config::parse::flag::parse_duration_opt, env = config::env::DAEMON_SESSION_IDLE_SHUTDOWN_GRACE)]
    session_idle_shutdown_grace: Option<Duration>,
}

impl DaemonServeCommand {
    async fn run(self, config: ConfigResolver) -> ExitCode {
        let sqlite_db = self.sqlite_db.unwrap_or_else(|| config.sqlite_db());
        let mut daemon_config = DaemonConfig::new(RuntimeDatabase::sqlite(sqlite_db))
            .with_session_lease_duration(config.daemon_session_lease_duration())
            .with_session_idle_shutdown_grace(config.daemon_session_idle_shutdown_grace());
        if let Some(root) = config.daemon_runtime_root() {
            daemon_config = daemon_config.with_runtime_root(root);
        }
        if let Some(duration) = self.session_lease_duration {
            daemon_config = daemon_config.with_session_lease_duration(duration);
        }
        if let Some(grace) = self.session_idle_shutdown_grace {
            daemon_config = daemon_config.with_session_idle_shutdown_grace(grace);
        }
        let daemon = Daemon::new(daemon_config);

        match daemon.serve().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => CommandFailure::report(err),
        }
    }
}

/// Show daemon status for the configured runtime instance
#[derive(Args, Debug)]
struct DaemonStatusCommand {
    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,
}

impl DaemonStatusCommand {
    async fn run(self, config: &ConfigResolver) -> ExitCode {
        match inspect_daemon(config).await {
            Ok(status) => {
                if let Err(err) = write_daemon_status(self.output, &status) {
                    CommandFailure::report(err)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(err) => CommandFailure::report(err),
        }
    }
}

/// Request daemon shutdown for the configured runtime instance
#[derive(Args, Debug)]
struct DaemonShutdownCommand {
    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,
}

impl DaemonShutdownCommand {
    async fn run(self, config: &ConfigResolver) -> ExitCode {
        match shutdown_daemon(config).await {
            Ok(result) => {
                if let Err(err) = write_daemon_status(self.output, result.status()) {
                    CommandFailure::report(err)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(err) => CommandFailure::report(err),
        }
    }
}

async fn inspect_daemon(config: &ConfigResolver) -> anyhow::Result<DaemonStatus> {
    FeedRuntime::new(config)?.inspect_daemon().await
}

async fn shutdown_daemon(config: &ConfigResolver) -> anyhow::Result<synd_runtime::ShutdownResult> {
    FeedRuntime::new(config)?.shutdown_daemon().await
}

fn write_daemon_status(format: OutputFormat, status: &DaemonStatus) -> anyhow::Result<()> {
    let status_output = DaemonStatusOutput::from(status);
    let mut stdout = io::stdout();

    status_output.write_as(format, &mut stdout)
}

fn daemon_state_label(state: DaemonState) -> &'static str {
    match state {
        DaemonState::Running => "running",
        DaemonState::NotRunning => "not-running",
    }
}

#[derive(Debug, Serialize)]
struct DaemonStatusOutput {
    state: &'static str,
    placement: DaemonPlacementOutput,
    sessions: Option<DaemonSessionsOutput>,
}

impl DaemonStatusOutput {
    fn write_as(&self, format: OutputFormat, writer: &mut impl io::Write) -> anyhow::Result<()> {
        match format {
            OutputFormat::Human => self.write_human(writer)?,
            OutputFormat::Json => {
                serde_json::to_writer_pretty(&mut *writer, self)?;
                writeln!(writer)?;
            }
        }

        Ok(())
    }

    fn write_human(&self, writer: &mut impl io::Write) -> io::Result<()> {
        writeln!(writer, "state: {}", self.state)?;
        writeln!(writer, "instance: {}", self.placement.runtime_instance_id)?;
        writeln!(
            writer,
            "runtime root: {}",
            self.placement.runtime_root.display()
        )?;
        writeln!(writer, "database: {}", self.placement.database.display())?;
        writeln!(writer, "endpoint: {}", self.placement.endpoint.display())?;
        writeln!(
            writer,
            "startup lock: {}",
            self.placement.startup_lock.display()
        )?;

        if let Some(sessions) = &self.sessions {
            sessions.write_human(writer)?;
        }

        Ok(())
    }
}

impl From<&DaemonStatus> for DaemonStatusOutput {
    fn from(status: &DaemonStatus) -> Self {
        let placement = status.placement();

        Self {
            state: daemon_state_label(status.state()),
            placement: DaemonPlacementOutput {
                runtime_root: placement.runtime_root().to_path_buf(),
                runtime_instance_id: placement.runtime_instance_id().to_owned(),
                database: placement.database().to_path_buf(),
                endpoint: placement.endpoint().to_path_buf(),
                startup_lock: placement.startup_lock().to_path_buf(),
            },
            sessions: status.sessions().map(DaemonSessionsOutput::from),
        }
    }
}

#[derive(Debug, Serialize)]
struct DaemonPlacementOutput {
    runtime_root: PathBuf,
    runtime_instance_id: String,
    database: PathBuf,
    endpoint: PathBuf,
    startup_lock: PathBuf,
}

#[derive(Debug, Serialize)]
struct DaemonSessionsOutput {
    active_sessions: usize,
    lease_duration: Duration,
    sweep_interval: Duration,
    idle_shutdown: DaemonIdleShutdownOutput,
}

impl DaemonSessionsOutput {
    fn write_human(&self, writer: &mut impl io::Write) -> io::Result<()> {
        writeln!(writer, "active sessions: {}", self.active_sessions)?;
        writeln!(
            writer,
            "session lease: {}",
            HumanDuration::from(self.lease_duration)
        )?;
        writeln!(
            writer,
            "session sweep interval: {}",
            HumanDuration::from(self.sweep_interval)
        )?;
        self.idle_shutdown.write_human(writer)
    }
}

impl From<&DaemonSessionStatus> for DaemonSessionsOutput {
    fn from(status: &DaemonSessionStatus) -> Self {
        Self {
            active_sessions: status.active_sessions(),
            lease_duration: status.lease_duration(),
            sweep_interval: status.sweep_interval(),
            idle_shutdown: DaemonIdleShutdownOutput::from(status.idle_shutdown()),
        }
    }
}

#[derive(Debug, Serialize)]
struct DaemonIdleShutdownOutput {
    enabled: bool,
    grace: Option<Duration>,
    pending: bool,
}

impl DaemonIdleShutdownOutput {
    fn write_human(&self, writer: &mut impl io::Write) -> io::Result<()> {
        let state = if self.enabled { "enabled" } else { "disabled" };
        writeln!(writer, "idle shutdown: {state}")?;
        if let Some(grace) = self.grace {
            writeln!(
                writer,
                "idle shutdown grace: {}",
                HumanDuration::from(grace)
            )?;
        }
        writeln!(writer, "idle shutdown pending: {}", self.pending)
    }
}

impl From<&DaemonIdleShutdownStatus> for DaemonIdleShutdownOutput {
    fn from(status: &DaemonIdleShutdownStatus) -> Self {
        Self {
            enabled: status.is_enabled(),
            grace: status.grace(),
            pending: status.is_pending(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn writes_runtime_placement_in_human_status() {
        let output = DaemonStatusOutput {
            state: "not-running",
            placement: DaemonPlacementOutput {
                runtime_root: PathBuf::from("/runtime"),
                runtime_instance_id: "runtime-1".to_owned(),
                database: PathBuf::from("/data/synd.db"),
                endpoint: PathBuf::from("/runtime/api.sock"),
                startup_lock: PathBuf::from("/runtime/api.lock"),
            },
            sessions: None,
        };
        let mut buffer = Vec::new();

        output.write_human(&mut buffer).unwrap();

        assert_eq!(
            String::from_utf8(buffer).unwrap(),
            "\
state: not-running
instance: runtime-1
runtime root: /runtime
database: /data/synd.db
endpoint: /runtime/api.sock
startup lock: /runtime/api.lock
"
        );
    }
}
