use std::{io, path::PathBuf, process::ExitCode, time::Duration};

use clap::{Args, Subcommand};
use serde::Serialize;
use synd_runtime::{Daemon, DaemonConfig, DaemonState, DaemonStatus, RuntimeDatabase};
use tracing::error;

use crate::{
    cli::OutputFormat,
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
            Err(err) => {
                error!("{err:?}");
                ExitCode::from(1)
            }
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
                    error!("{err:?}");
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(err) => {
                error!("{err:?}");
                ExitCode::from(1)
            }
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
                    error!("{err:?}");
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(err) => {
                error!("{err:?}");
                ExitCode::from(1)
            }
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
        writeln!(writer, "database: {}", self.placement.database.display())?;
        writeln!(writer, "endpoint: {}", self.placement.endpoint.display())
    }
}

impl From<&DaemonStatus> for DaemonStatusOutput {
    fn from(status: &DaemonStatus) -> Self {
        let placement = status.placement();

        Self {
            state: daemon_state_label(status.state()),
            placement: DaemonPlacementOutput {
                runtime_instance_id: placement.runtime_instance_id().to_owned(),
                database: placement.database().to_path_buf(),
                endpoint: placement.endpoint().to_path_buf(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct DaemonPlacementOutput {
    runtime_instance_id: String,
    database: PathBuf,
    endpoint: PathBuf,
}
