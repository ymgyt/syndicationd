use std::path::Path;

use synd_stdx::conf::Entry;
use thiserror::Error;

use crate::{
    cli::KvsdOptions,
    config::{
        file::{ConfigFile, ConfigFileError},
        kvsd, Config,
    },
};

#[derive(Error, Debug)]
pub enum ConfigResolverError {
    #[error("`{path}` {source}")]
    OpenConfigFile {
        path: String,
        source: ConfigFileError,
    },
}

/// The `ConfigResolver` is responsible for resolving the final configuration to be used
/// taking into account command line arguments, environment variables, configuration file and default values.
#[expect(dead_code)]
pub struct ConfigResolver {
    args: KvsdOptions,
}

impl ConfigResolver {
    pub fn from_args(args: KvsdOptions) -> Self {
        Self { args }
    }

    pub fn resolve(self) -> Result<Config, ConfigResolverError> {
        Ok(Config {
            connections_limit: Entry::with_default(kvsd::default::CONNECTIONS_LIMIT),
            buffer_size_per_connection: Entry::with_default(
                kvsd::default::BUFFER_SIZE_PER_CONNECTION,
            ),
            authenticate_timeout: Entry::with_default(kvsd::default::AUTHENTICATE_TIMEOUT),
            bind_address: Entry::with_default(kvsd::default::bind_address()),
            bind_port: Entry::with_default(kvsd::default::BIND_PORT),
            tls: Entry::with_default(kvsd::default::TLS_CONNECTION),
            root_dir: Entry::with_default(kvsd::default::root_dir()),
        })
    }

    #[expect(dead_code)]
    fn read_config_file<P: AsRef<Path>>(
        path: Option<P>,
    ) -> Result<Option<ConfigFile>, ConfigResolverError> {
        // TODO: abstruct filesystem
        if let Some(path) = path {
            Ok(Some(ConfigFile::load(path.as_ref()).map_err(|err| {
                ConfigResolverError::OpenConfigFile {
                    path: path.as_ref().to_string_lossy().to_string(),
                    source: err,
                }
            })?))
        } else {
            Ok(None)
        }
    }
}
