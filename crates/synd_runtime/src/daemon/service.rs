use crate::{Error, Result, RuntimeDatabase};

#[derive(Debug, Clone)]
pub struct Daemon {
    config: Config,
}

impl Daemon {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    #[allow(clippy::unused_async)]
    pub async fn serve(self) -> Result<()> {
        Err(Error::NotImplemented("Daemon::serve"))
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    database: RuntimeDatabase,
}

impl Config {
    pub fn new(database: RuntimeDatabase) -> Self {
        Self { database }
    }

    pub fn database(&self) -> &RuntimeDatabase {
        &self.database
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchConfig {
    _private: (),
}
