use std::{io, path::Path, process::ExitCode};

use anyhow::Context;
use clap::Args;
use synd_o11y::health_check::Health;

use crate::{
    cli::{
        BackendMode,
        port::{AuthMode, PortContext},
    },
    config::ConfigResolver,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum CheckFormat {
    Human,
    Json,
}

/// Check application conditions
#[derive(Args, Debug)]
pub struct CheckCommand {
    #[arg(value_enum, long, default_value_t = CheckFormat::Human)]
    pub format: CheckFormat,
}

impl CheckCommand {
    #[allow(clippy::unused_self)]
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        if let Err(err) = self.check(config).await {
            tracing::error!("{err:?}");
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }

    async fn check(self, config: ConfigResolver) -> anyhow::Result<()> {
        let Self { format } = self;
        let backend_mode = config.backend_mode();
        let local_sqlite_db = match backend_mode {
            BackendMode::Local => Some(config.local_sqlite_db()),
            BackendMode::Remote => None,
        };
        let endpoint = match backend_mode {
            BackendMode::Local => None,
            BackendMode::Remote => Some(config.api_endpoint()),
        };
        let cx = PortContext::new(&config, AuthMode::None).await?;

        let api_health = cx
            .client
            .health()
            .await
            .context("api health check")
            .inspect_err(|err| eprintln!("{err:?}"))
            .ok();

        let cache_dir = config.cache_dir();
        let log_path = config.log_file();
        let config_path = config.config_file();

        match format {
            CheckFormat::Human => {
                Self::print(
                    io::stdout(),
                    CheckOutput {
                        health: api_health,
                        config_path: &config_path,
                        cache_dir: &cache_dir,
                        log_path: log_path.as_path(),
                        backend_mode,
                        local_sqlite_db: local_sqlite_db.as_deref(),
                        endpoint: endpoint.as_ref(),
                    },
                )?;
            }
            CheckFormat::Json => {
                let health = match api_health {
                    Some(health) => serde_json::json!(&health),
                    None => serde_json::json!("unknown"),
                };
                println!(
                    "{}",
                    serde_json::json!({
                        "api": health,
                        "config": config_path.display().to_string(),
                        "cache": cache_dir.display().to_string(),
                        "log": log_path.display().to_string(),
                        "backend": Self::backend_mode_name(backend_mode),
                        "local_sqlite_db": local_sqlite_db.map(|path| path.display().to_string()),
                        "endpoint": endpoint.map(|endpoint| endpoint.to_string()),
                    })
                );
            }
        }

        Ok(())
    }

    fn print(mut writer: impl io::Write, output: CheckOutput<'_>) -> io::Result<()> {
        let w = &mut writer;
        let CheckOutput {
            health,
            config_path,
            cache_dir,
            log_path,
            backend_mode,
            local_sqlite_db,
            endpoint,
        } = output;

        writeln!(
            w,
            " Api Health: {}",
            health
                .as_ref()
                .map_or("unknown".into(), |h| h.status.to_string())
        )?;
        writeln!(
            w,
            "Api Version: {}",
            health.and_then(|h| h.version).unwrap_or("unknown".into())
        )?;
        writeln!(w, "    Backend: {}", Self::backend_mode_name(backend_mode))?;
        if let Some(local_sqlite_db) = local_sqlite_db {
            writeln!(w, "  SQLite DB: {}", local_sqlite_db.display())?;
        }
        if let Some(endpoint) = endpoint {
            writeln!(w, "   Endpoint: {endpoint}")?;
        }

        writeln!(w, "     Config: {}", config_path.display())?;
        writeln!(w, "      Cache: {}", cache_dir.display())?;
        writeln!(w, "        Log: {}", log_path.display())?;
        Ok(())
    }

    const fn backend_mode_name(backend_mode: BackendMode) -> &'static str {
        match backend_mode {
            BackendMode::Remote => "remote",
            BackendMode::Local => "local",
        }
    }
}

struct CheckOutput<'a> {
    health: Option<Health>,
    config_path: &'a Path,
    cache_dir: &'a Path,
    log_path: &'a Path,
    backend_mode: BackendMode,
    local_sqlite_db: Option<&'a Path>,
    endpoint: Option<&'a url::Url>,
}
