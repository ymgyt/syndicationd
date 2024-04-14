use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::ProjectDirs;

mod categories;
pub use categories::{Categories, Icon, IconColor};

pub mod api {
    pub const ENDPOINT: &str = "https://api.syndicationd.ymgyt.io:6100";
}

pub mod env {
    macro_rules! env_key {
        ($key:expr) => {
            concat!("SYND", "_", $key)
        };
    }
    /// Log directive
    pub const LOG_DIRECTIVE: &str = env_key!("LOG");

    pub const ENDPOINT: &str = env_key!("ENDPOINT");
    pub const LOG_PATH: &str = env_key!("LOG");
    pub const THEME: &str = env_key!("THEME");
}

pub mod client {
    pub const DEFAULT_TIMEOUT: &str = "30s";
    pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    /// Number of entries to fetch
    pub const INITIAL_ENTRIES_TO_FETCH: i64 = 200;
    /// Number of feeds to fetch
    pub const INITIAL_FEEDS_TO_FETCH: i64 = 50;
}

pub fn cache_dir() -> &'static Path {
    project_dirs().cache_dir()
}

pub fn log_path() -> PathBuf {
    project_dirs().data_dir().join("synd.log")
}

fn project_dirs() -> &'static ProjectDirs {
    static PROJECT_DIRS: OnceLock<ProjectDirs> = OnceLock::new();

    PROJECT_DIRS.get_or_init(|| {
        ProjectDirs::from("ymgyt.io", "syndicationd", "synd").expect("Failed to get project dirs")
    })
}
