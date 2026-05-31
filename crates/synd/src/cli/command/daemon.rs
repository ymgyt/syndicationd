use std::{path::PathBuf, process::ExitCode};

use clap::{Args, Subcommand};
use synd_runtime::{Daemon, DaemonConfig, DaemonState, DaemonStatus, RuntimeDatabase};
use tracing::error;

use crate::{config::ConfigResolver, runtime::FeedRuntime};

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
}

impl DaemonServeCommand {
    async fn run(self, config: ConfigResolver) -> ExitCode {
        let sqlite_db = self.sqlite_db.unwrap_or_else(|| config.sqlite_db());
        let daemon = Daemon::new(DaemonConfig::new(RuntimeDatabase::sqlite(sqlite_db)));

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
struct DaemonStatusCommand {}

impl DaemonStatusCommand {
    async fn run(self, config: &ConfigResolver) -> ExitCode {
        match inspect_daemon(config).await {
            Ok(status) => {
                print_daemon_status(&status);
                ExitCode::SUCCESS
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
struct DaemonShutdownCommand {}

impl DaemonShutdownCommand {
    async fn run(self, config: &ConfigResolver) -> ExitCode {
        match shutdown_daemon(config).await {
            Ok(result) => {
                print_daemon_status(result.status());
                ExitCode::SUCCESS
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

fn print_daemon_status(status: &DaemonStatus) {
    let placement = status.placement();

    println!("state: {}", daemon_state_label(status.state()));
    println!("instance: {}", placement.runtime_instance_id());
    println!("database: {}", placement.database().display());
    println!("endpoint: {}", placement.endpoint().display());
}

fn daemon_state_label(state: DaemonState) -> &'static str {
    match state {
        DaemonState::Running => "running",
        DaemonState::NotRunning => "not-running",
    }
}
