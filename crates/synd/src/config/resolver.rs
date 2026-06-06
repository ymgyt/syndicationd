use std::{
    io::{self, ErrorKind},
    path::PathBuf,
    time::Duration,
};

use synd_support::{
    conf::Entry,
    fs::{FileSystem, fsimpl},
};
use thiserror::Error;
use tracing::debug;

use crate::{
    cli::{self, ApiOptions, BackendOptions, DaemonOptions, FeedOptions, GithubOptions},
    config::{
        self,
        file::{ConfigFile, ConfigFileError},
    },
};
use synd_term::keymap::{CompiledKeymaps, KeymapError};
use synd_term::{config::Categories, ui::theme::Palette};

/// `ConfigResolver` is responsible for resolving the application's configuration
/// while taking priority into account.
/// Specifically, it takes the following elements into account
/// with the first elements having the highest priority
/// * command line arguments
/// * environment variables
/// * configuration file
/// * default values
#[derive(Debug)]
pub struct ConfigResolver {
    config_file: PathBuf,
    log_file: Entry<PathBuf>,
    cache_dir: Entry<PathBuf>,
    sqlite_db: Entry<PathBuf>,
    api_timeout: Entry<Duration>,
    daemon_runtime_root: Entry<Option<PathBuf>>,
    daemon_session_lease_duration: Entry<Duration>,
    daemon_session_idle_shutdown_grace: Entry<Duration>,
    feed_entries_limit: Entry<usize>,
    feed_browser_command: Entry<PathBuf>,
    feed_browser_args: Entry<Vec<String>>,
    github_enable: Entry<bool>,
    github_pat: Entry<String>,
    palette: Entry<Palette>,
    categories: Categories,
    keymaps: CompiledKeymaps,
}

impl ConfigResolver {
    pub fn builder() -> ConfigResolverBuilder {
        ConfigResolverBuilder::default()
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_file.clone()
    }

    pub fn log_file(&self) -> PathBuf {
        self.log_file.resolve_ref().clone()
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache_dir.resolve_ref().clone()
    }

    pub fn sqlite_db(&self) -> PathBuf {
        self.sqlite_db.resolve_ref().clone()
    }

    pub fn api_timeout(&self) -> Duration {
        self.api_timeout.resolve()
    }

    pub fn daemon_runtime_root(&self) -> Option<PathBuf> {
        self.daemon_runtime_root.resolve_ref().clone()
    }

    pub fn daemon_session_lease_duration(&self) -> Duration {
        self.daemon_session_lease_duration.resolve()
    }

    pub fn daemon_session_idle_shutdown_grace(&self) -> Duration {
        self.daemon_session_idle_shutdown_grace.resolve()
    }

    pub fn feed_entries_limit(&self) -> usize {
        self.feed_entries_limit.resolve()
    }

    pub fn feed_browser_command(&self) -> PathBuf {
        self.feed_browser_command.resolve_ref().clone()
    }

    pub fn feed_browser_args(&self) -> Vec<String> {
        self.feed_browser_args.resolve_ref().clone()
    }

    pub fn is_github_enable(&self) -> bool {
        self.github_enable.resolve()
    }

    pub fn github_pat(&self) -> String {
        self.github_pat.resolve_ref().clone()
    }

    pub fn palette(&self) -> Palette {
        self.palette.resolve_ref().clone()
    }

    pub fn categories(&self) -> Categories {
        self.categories.clone()
    }

    pub fn keymaps(&self) -> CompiledKeymaps {
        self.keymaps.clone()
    }
}

impl ConfigResolver {
    /// performs validation based on the relationshsips between the various settings.
    fn validate(self) -> Result<Self, ConfigResolverBuildError> {
        if self.github_enable.resolve() && self.github_pat.resolve_ref().is_empty() {
            return Err(ConfigResolverBuildError::ValidateConfigFile(
                "github pat is required for github feature".into(),
            ));
        }
        Ok(self)
    }
}

#[derive(Error, Debug)]
pub enum ConfigResolverBuildError {
    #[error("failed to open {path} {err}")]
    ConfigFileOpen { path: String, err: io::Error },
    #[error(transparent)]
    ConfigFileLoad(#[from] ConfigFileError),
    #[error("invalid configuration: {0}")]
    ValidateConfigFile(String),
    #[error("invalid keymap configuration: {0}")]
    Keymap(#[from] KeymapError),
}

#[derive(Default)]
pub struct ConfigResolverBuilder<FS = fsimpl::FileSystem> {
    config_file: Option<PathBuf>,
    log_file_flag: Option<PathBuf>,
    cache_dir_flag: Option<PathBuf>,
    api_flags: Option<ApiOptions>,
    daemon_flags: Option<DaemonOptions>,
    backend_flags: Option<BackendOptions>,
    feed_flags: Option<FeedOptions>,
    github_flags: Option<GithubOptions>,
    palette_flag: Option<cli::Palette>,
    fs: FS,
}

impl ConfigResolverBuilder {
    #[must_use]
    pub fn config_file(self, config_file: Option<PathBuf>) -> Self {
        Self {
            config_file,
            ..self
        }
    }

    #[must_use]
    pub fn log_file(self, log_file_flag: Option<PathBuf>) -> Self {
        Self {
            log_file_flag,
            ..self
        }
    }

    #[must_use]
    pub fn cache_dir(self, cache_dir_flag: Option<PathBuf>) -> Self {
        Self {
            cache_dir_flag,
            ..self
        }
    }

    #[must_use]
    pub fn api_options(self, api_options: ApiOptions) -> Self {
        Self {
            api_flags: Some(api_options),
            ..self
        }
    }

    #[must_use]
    pub fn daemon_options(self, daemon_options: DaemonOptions) -> Self {
        Self {
            daemon_flags: Some(daemon_options),
            ..self
        }
    }

    #[must_use]
    pub fn backend_options(self, backend_options: BackendOptions) -> Self {
        Self {
            backend_flags: Some(backend_options),
            ..self
        }
    }

    #[must_use]
    pub fn feed_options(self, feed_options: FeedOptions) -> Self {
        Self {
            feed_flags: Some(feed_options),
            ..self
        }
    }

    #[must_use]
    pub fn github_options(self, github_options: GithubOptions) -> Self {
        Self {
            github_flags: Some(github_options),
            ..self
        }
    }

    #[must_use]
    pub fn palette(self, palette: Option<cli::Palette>) -> Self {
        Self {
            palette_flag: palette,
            ..self
        }
    }

    pub fn try_build(self) -> Result<ConfigResolver, ConfigResolverBuildError> {
        let (mut config_file, config_path) = self.load_config_file()?;

        // construct categories
        let mut categories = Categories::default_toml();
        if let Some(user_defined) = config_file.as_mut().and_then(|c| c.categories.take()) {
            categories.merge(user_defined);
        }

        let user_keymaps = config_file
            .as_mut()
            .and_then(|config| config.keys.take())
            .unwrap_or_default();
        let keymaps = CompiledKeymaps::default_with_user_config(user_keymaps)?;

        let ConfigResolverBuilder {
            api_flags: Some(ApiOptions { client_timeout }),
            daemon_flags: Some(daemon_flags),
            backend_flags: Some(BackendOptions { sqlite_db }),
            feed_flags:
                Some(FeedOptions {
                    entries_limit,
                    browser,
                    browser_args,
                }),
            github_flags:
                Some(GithubOptions {
                    enable_github_notification,
                    github_pat,
                }),
            log_file_flag,
            cache_dir_flag,
            palette_flag,
            ..
        } = self
        else {
            panic!()
        };

        let daemon_entries = DaemonConfigEntries::from_sources(&mut config_file, daemon_flags);
        let resolver = ConfigResolver {
            config_file: config_path,
            log_file: Entry::with_default(config::log_path())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.log.as_mut())
                        .and_then(|log| log.path.take()),
                )
                .with_flag(log_file_flag),
            cache_dir: Entry::with_default(config::cache::dir().to_owned())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.cache.as_mut())
                        .and_then(|cache| cache.directory.take()),
                )
                .with_flag(cache_dir_flag),
            sqlite_db: Entry::with_default(config::local::sqlite_db())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.backend.as_mut())
                        .and_then(|backend| backend.sqlite_db.take()),
                )
                .with_flag(sqlite_db),
            api_timeout: Entry::with_default(config::client::DEFAULT_TIMEOUT)
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.api.as_mut())
                        .and_then(|api| api.timeout.take()),
                )
                .with_flag(client_timeout),
            daemon_runtime_root: daemon_entries.runtime_root,
            daemon_session_lease_duration: daemon_entries.session_lease_duration,
            daemon_session_idle_shutdown_grace: daemon_entries.session_idle_shutdown_grace,

            feed_entries_limit: Entry::with_default(config::feed::DEFAULT_ENTRIES_LIMIT)
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.feed.as_mut())
                        .and_then(|feed| feed.entries_limit),
                )
                .with_flag(entries_limit),
            feed_browser_command: Entry::with_default(config::feed::default_browser_command())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.feed.as_mut())
                        .and_then(|feed| feed.browser.as_mut())
                        .and_then(|browser| browser.command.take()),
                )
                .with_flag(browser),

            feed_browser_args: Entry::with_default(Vec::new())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.feed.as_mut())
                        .and_then(|feed| feed.browser.as_mut())
                        .and_then(|browser| browser.args.take()),
                )
                .with_flag(browser_args),

            github_enable: Entry::with_default(false)
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.github.as_mut())
                        .and_then(|gh| gh.enable.take()),
                )
                .with_flag(enable_github_notification),
            github_pat: Entry::with_default(String::new())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.github.as_mut())
                        .and_then(|gh| gh.pat.take()),
                )
                .with_flag(github_pat),
            palette: Entry::with_default(config::theme::DEFAULT_PALETTE.into())
                .with_file(
                    config_file
                        .as_mut()
                        .and_then(|c| c.theme.as_mut())
                        .and_then(|theme| theme.name.take())
                        .map(Into::into),
                )
                .with_flag(palette_flag.map(Into::into)),
            categories,
            keymaps,
        };

        resolver.validate()
    }

    fn load_config_file(&self) -> Result<(Option<ConfigFile>, PathBuf), ConfigResolverBuildError> {
        if let Some(path) = &self.config_file {
            // If a configuration file path is explicitly specified, search for that file
            // and return an error if it is not found.
            return match self.fs.open_file(path) {
                Ok(f) => Ok((Some(ConfigFile::new(f)?), path.clone())),
                Err(err) => Err(ConfigResolverBuildError::ConfigFileOpen {
                    path: path.display().to_string(),
                    err,
                }),
            };
        }

        // If the path is not specified, builder search for the default path
        // but will not return an error even if it is not found.
        let default_path = config::config_path();
        match self.fs.open_file(&default_path) {
            Ok(f) => Ok((Some(ConfigFile::new(f)?), default_path)),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {
                    debug!(path = %default_path.display(), "default config file not found");
                    Ok((None, default_path))
                }
                _ => Err(ConfigResolverBuildError::ConfigFileOpen {
                    path: default_path.display().to_string(),
                    err,
                }),
            },
        }
    }
}

#[derive(Debug)]
struct DaemonConfigEntries {
    runtime_root: Entry<Option<PathBuf>>,
    session_lease_duration: Entry<Duration>,
    session_idle_shutdown_grace: Entry<Duration>,
}

impl DaemonConfigEntries {
    fn from_sources(config_file: &mut Option<ConfigFile>, flags: DaemonOptions) -> Self {
        let DaemonOptions {
            runtime_root,
            daemon_session_lease_duration,
            daemon_session_idle_shutdown_grace,
        } = flags;
        let file_entry = config_file.as_mut().and_then(|c| c.daemon.as_mut());
        let (runtime_root_file, session_lease_duration_file, session_idle_shutdown_grace_file) =
            match file_entry {
                Some(daemon) => (
                    daemon.runtime_root.take(),
                    daemon.session_lease_duration.take(),
                    daemon.session_idle_shutdown_grace.take(),
                ),
                None => (None, None, None),
            };

        Self {
            runtime_root: Entry::with_default(None)
                .with_file(runtime_root_file.map(Some))
                .with_flag(runtime_root.map(Some)),
            session_lease_duration: Entry::with_default(
                config::daemon::default_session_lease_duration(),
            )
            .with_file(session_lease_duration_file)
            .with_flag(daemon_session_lease_duration),
            session_idle_shutdown_grace: Entry::with_default(
                config::daemon::default_session_idle_shutdown_grace(),
            )
            .with_file(session_idle_shutdown_grace_file)
            .with_flag(daemon_session_idle_shutdown_grace),
        }
    }
}
