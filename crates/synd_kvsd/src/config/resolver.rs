use std::path::Path;

use thiserror::Error;

use crate::{
    args::KvsdOptions,
    config::{file::ConfigFile, Config},
};

#[derive(Error, Debug)]
pub enum ConfigResolverError {}

/// The `ConfigResolver` is responsible for resolving the final configuration to be used
/// taking into account command line arguments, environment variables, configuration file and default values.
pub struct ConfigResolver {
    args: KvsdOptions,
}

impl ConfigResolver {
    pub fn new(args: KvsdOptions) -> Self {
        Self { args }
    }

    pub fn resolve(self) -> Result<Config, ConfigResolverError> {
        let mut config = Config::default();

        if let Some(file) = Self::read_config_file(self.args.config.as_ref())? {
            config.merge_config_file(file);
        }

        config.merge_args(self.args);
        Ok(config)
    }

    fn read_config_file<P: AsRef<Path>>(
        _path: Option<P>,
    ) -> Result<Option<ConfigFile>, ConfigResolverError> {
        Ok(None)
    }
}
