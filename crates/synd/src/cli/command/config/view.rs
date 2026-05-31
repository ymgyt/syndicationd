use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use clap::Args;
use serde::Serialize;
use tracing::error;

use crate::{cli::OutputFormat, config::ConfigResolver};

/// View resolved configuration
#[derive(Args, Debug)]
pub struct ConfigViewCommand {
    /// Output format
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Human)]
    output: OutputFormat,
}

impl ConfigViewCommand {
    pub fn run(self, config: &ConfigResolver) -> ExitCode {
        if let Err(err) = self.view(config) {
            error!("{err:?}");
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }

    fn view(self, config: &ConfigResolver) -> anyhow::Result<()> {
        let output = ConfigViewOutput::from_config(config);
        let mut stdout = io::stdout();

        match self.output {
            OutputFormat::Human => output.print(&mut stdout)?,
            OutputFormat::Json => {
                serde_json::to_writer_pretty(&mut stdout, &output)?;
                writeln!(stdout)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct ConfigViewOutput {
    config: ConfigFileOutput,
    cache: CacheOutput,
    log: LogOutput,
    backend: BackendOutput,
    api: ApiOutput,
    feed: FeedOutput,
    github: GithubOutput,
    theme: ThemeOutput,
}

#[derive(Debug, Serialize)]
struct ConfigFileOutput {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
struct CacheOutput {
    directory: PathBuf,
}

#[derive(Debug, Serialize)]
struct LogOutput {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
struct BackendOutput {
    sqlite_db: PathBuf,
}

#[derive(Debug, Serialize)]
struct ApiOutput {
    timeout: String,
}

#[derive(Debug, Serialize)]
struct FeedOutput {
    entries_limit: usize,
    browser: BrowserOutput,
}

#[derive(Debug, Serialize)]
struct BrowserOutput {
    command: Option<PathBuf>,
    args: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GithubOutput {
    enabled: bool,
    pat_configured: bool,
}

#[derive(Debug, Serialize)]
struct ThemeOutput {
    name: &'static str,
}

impl ConfigViewOutput {
    fn from_config(config: &ConfigResolver) -> Self {
        let browser_command = config.feed_browser_command();
        let browser_command = if browser_command.as_os_str().is_empty() {
            None
        } else {
            Some(browser_command)
        };

        Self {
            config: ConfigFileOutput {
                path: config.config_file(),
            },
            cache: CacheOutput {
                directory: config.cache_dir(),
            },
            log: LogOutput {
                path: config.log_file(),
            },
            backend: BackendOutput {
                sqlite_db: config.sqlite_db(),
            },
            api: ApiOutput {
                timeout: format_duration(config.api_timeout()),
            },
            feed: FeedOutput {
                entries_limit: config.feed_entries_limit(),
                browser: BrowserOutput {
                    command: browser_command,
                    args: config.feed_browser_args(),
                },
            },
            github: GithubOutput {
                enabled: config.is_github_enable(),
                pat_configured: !config.github_pat().is_empty(),
            },
            theme: ThemeOutput {
                name: config.palette().name(),
            },
        }
    }

    fn print(&self, mut writer: impl io::Write) -> io::Result<()> {
        writeln!(writer, "     Config: {}", self.config.path.display())?;
        writeln!(writer, "      Cache: {}", self.cache.directory.display())?;
        writeln!(writer, "        Log: {}", self.log.path.display())?;
        writeln!(writer, "  SQLite DB: {}", self.backend.sqlite_db.display())?;
        writeln!(writer, "    Timeout: {}", self.api.timeout)?;
        writeln!(writer, " Feed Limit: {}", self.feed.entries_limit)?;
        writeln!(
            writer,
            "    Browser: {}",
            path_or_not_set(self.feed.browser.command.as_deref())
        )?;
        if !self.feed.browser.args.is_empty() {
            writeln!(writer, "Browser Arg: {}", self.feed.browser.args.join(" "))?;
        }
        writeln!(writer, "      Theme: {}", self.theme.name)?;
        writeln!(
            writer,
            "     GitHub: {}",
            if self.github.enabled {
                "enabled"
            } else {
                "disabled"
            }
        )?;
        writeln!(
            writer,
            " GitHub PAT: {}",
            if self.github.pat_configured {
                "set"
            } else {
                "not set"
            }
        )?;
        Ok(())
    }
}

fn path_or_not_set(path: Option<&Path>) -> String {
    path.map_or_else(|| "not set".to_owned(), |path| path.display().to_string())
}

fn format_duration(duration: Duration) -> String {
    format!("{duration:?}")
}
