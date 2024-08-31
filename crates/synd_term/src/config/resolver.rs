use std::{
    io::{self, ErrorKind},
    path::PathBuf,
    time::Duration,
};

use thiserror::Error;
use url::Url;

use crate::{
    cli::{self, ApiOptions, FeedOptions, GithubOptions},
    config::{
        self,
        file::{ConfigFile, ConfigFileError},
        Categories,
    },
    filesystem::{fsimpl, FileSystem},
    ui::theme::Palette,
};

/// `ConfigResolver` is responsible for resolving the application's configration
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
    api_endpoint: Entry<Url>,
    api_timeout: Entry<Duration>,
    feed_entries_limit: Entry<usize>,
    feed_browser_command: Entry<PathBuf>,
    feed_browser_args: Entry<Vec<String>>,
    github_enable: Entry<bool>,
    github_pat: Entry<String>,
    palette: Entry<Palette>,
    categories: Categories,
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

    pub fn api_endpoint(&self) -> Url {
        self.api_endpoint.resolve_ref().clone()
    }

    pub fn api_timeout(&self) -> Duration {
        self.api_timeout.resolve()
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
    #[error("invalid configration: {0}")]
    ValidateConfigFile(String),
}

#[derive(Default)]
pub struct ConfigResolverBuilder<FS = fsimpl::FileSystem> {
    config_file: Option<PathBuf>,
    log_file_flag: Option<PathBuf>,
    cache_dir_flag: Option<PathBuf>,
    api_flags: Option<ApiOptions>,
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

    pub fn build(self) -> ConfigResolver {
        self.try_build().expect("failed to build config resolver")
    }

    pub fn try_build(self) -> Result<ConfigResolver, ConfigResolverBuildError> {
        let (mut config_file, config_path) = if let Some(path) = self.config_file {
            // If a configuration file path is explicitly specified, search for that file
            // and return an error if it is not found.
            match self.fs.open_file(&path) {
                Ok(f) => (Some(ConfigFile::new(f)?), path),
                Err(err) => {
                    return Err(ConfigResolverBuildError::ConfigFileOpen {
                        path: path.display().to_string(),
                        err,
                    })
                }
            }
        // If the path is not specified, builder search for the default path
        // but will not return an error even if it is not found.
        } else {
            let default_path = config::config_path();
            match self.fs.open_file(&default_path) {
                Ok(f) => (Some(ConfigFile::new(f)?), default_path),
                Err(err) => match err.kind() {
                    ErrorKind::NotFound => {
                        tracing::debug!(path = %default_path.display(), "default config file not found");
                        (None, default_path)
                    }
                    _ => {
                        return Err(ConfigResolverBuildError::ConfigFileOpen {
                            path: default_path.display().to_string(),
                            err,
                        })
                    }
                },
            }
        };

        // construct categories
        let mut categories = Categories::default_toml();
        if let Some(user_defined) = config_file.as_mut().and_then(|c| c.categories.take()) {
            categories.merge(user_defined);
        }

        let ConfigResolverBuilder {
            api_flags:
                Some(ApiOptions {
                    endpoint,
                    client_timeout,
                }),
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

        let resolver = ConfigResolver {
            config_file: config_path,
            log_file: Entry {
                flag: log_file_flag,
                default: config::log_path(),
                file: config_file
                    .as_mut()
                    .and_then(|c| c.log.as_mut())
                    .and_then(|log| log.path.take()),
            },
            cache_dir: Entry {
                flag: cache_dir_flag,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.cache.as_mut())
                    .and_then(|cache| cache.directory.take()),
                default: config::cache::dir().to_owned(),
            },
            api_endpoint: Entry {
                flag: endpoint,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.api.as_mut())
                    .and_then(|api| api.endpoint.take()),
                default: Url::parse(config::api::ENDPOINT).unwrap(),
            },
            api_timeout: Entry {
                flag: client_timeout,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.api.as_mut())
                    .and_then(|api| api.timeout.take()),
                default: config::client::DEFAULT_TIMEOUT,
            },
            feed_entries_limit: Entry {
                flag: entries_limit,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.feed.as_mut())
                    .and_then(|feed| feed.entries_limit),
                default: config::feed::DEFAULT_ENTRIES_LIMIT,
            },
            feed_browser_command: Entry {
                flag: browser,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.feed.as_mut())
                    .and_then(|feed| feed.browser.as_mut())
                    .and_then(|brower| brower.command.take()),
                default: config::feed::default_brower_command(),
            },
            feed_browser_args: Entry {
                flag: browser_args,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.feed.as_mut())
                    .and_then(|feed| feed.browser.as_mut())
                    .and_then(|brower| brower.args.take()),
                default: Vec::new(),
            },
            github_enable: Entry {
                flag: enable_github_notification,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.github.as_mut())
                    .and_then(|gh| gh.enable.take()),
                default: false,
            },
            github_pat: Entry {
                flag: github_pat,
                file: config_file
                    .as_mut()
                    .and_then(|c| c.github.as_mut())
                    .and_then(|gh| gh.pat.take()),
                default: String::new(),
            },
            palette: Entry {
                flag: palette_flag.map(Into::into),
                file: config_file
                    .as_mut()
                    .and_then(|c| c.theme.as_mut())
                    .and_then(|theme| theme.name.take())
                    .map(Into::into),
                default: config::theme::DEFAULT_PALETTE.into(),
            },
            categories,
        };

        resolver.validate()
    }
}

#[derive(Debug)]
struct Entry<T> {
    flag: Option<T>,
    file: Option<T>,
    default: T,
}

impl<T> Entry<T> {
    fn resolve_ref(&self) -> &T {
        self.flag
            .as_ref()
            .or(self.file.as_ref())
            .unwrap_or(&self.default)
    }
}

impl<T> Entry<T>
where
    T: Copy,
{
    fn resolve(&self) -> T {
        *self.resolve_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_order() {
        let flag = 10;
        let file = 9;
        let default = 8;

        let e = Entry {
            flag: Some(flag),
            file: Some(file),
            default,
        };
        assert_eq!(e.resolve(), flag, "flag should have highest priority");

        let e = Entry {
            flag: None,
            file: Some(file),
            default,
        };
        assert_eq!(
            e.resolve(),
            file,
            "file should have higher priority over default"
        );

        let e = Entry {
            flag: None,
            file: None,
            default,
        };
        assert_eq!(e.resolve(), default);
    }
}
