use std::{path::PathBuf, sync::OnceLock};

use directories::ProjectDirs;

mod file;
pub use file::INIT_CONFIG;
pub(crate) mod parse;

mod resolver;
pub use resolver::ConfigResolver;

pub mod env {
    macro_rules! env_key {
        ($key:expr) => {
            concat!("SYND", "_", $key)
        };
    }

    pub const LOG_DIRECTIVE: &str = env_key!("LOG");
    pub const CLIENT_TIMEOUT: &str = env_key!("CLIENT_TIMEOUT");
    pub const CONFIG_FILE: &str = env_key!("CONFIG_FILE");
    pub const LOG_FILE: &str = env_key!("LOG_FILE");
    pub const CACHE_DIR: &str = env_key!("CACHE_DIR");
    pub const SQLITE_DB: &str = env_key!("SQLITE_DB");
    pub const THEME: &str = env_key!("THEME");
    pub const FEED_ENTRIES_LIMIT: &str = env_key!("ENTRIES_LIMIT");
    pub const FEED_BROWSER: &str = env_key!("BROWSER");
    pub const FEED_BROWSER_ARGS: &str = env_key!("BROWSER_ARGS");
    pub const ENABLE_GITHUB: &str = env_key!("ENABLE_GH");
    pub const GITHUB_PAT: &str = env_key!("GH_PAT");
}

pub mod client {
    use std::time::Duration;

    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
}

pub mod feed {
    use std::path::PathBuf;

    pub const DEFAULT_ENTRIES_LIMIT: usize = 200;

    pub fn default_browser_command() -> PathBuf {
        PathBuf::new()
    }
}

pub mod cache {
    use std::path::Path;

    pub fn dir() -> &'static Path {
        super::project_dirs().cache_dir()
    }
}

pub mod local {
    use std::path::PathBuf;

    pub fn sqlite_db() -> PathBuf {
        super::project_dirs().data_dir().join("synd.db")
    }
}

pub(crate) mod theme {
    use crate::cli::Palette;

    pub(crate) const DEFAULT_PALETTE: Palette = Palette::Ferra;
}

pub fn log_path() -> PathBuf {
    project_dirs().data_dir().join("synd.log")
}

pub fn config_path() -> PathBuf {
    project_dirs().config_dir().join("config.toml")
}

fn project_dirs() -> &'static ProjectDirs {
    static PROJECT_DIRS: OnceLock<ProjectDirs> = OnceLock::new();

    PROJECT_DIRS.get_or_init(|| {
        ProjectDirs::from("", "", "syndicationd").expect("Failed to get project dirs")
    })
}
