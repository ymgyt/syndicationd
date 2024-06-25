use std::{path::PathBuf, sync::OnceLock};

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
    pub const LOG_PATH: &str = env_key!("LOG_PATH");
    pub const THEME: &str = env_key!("THEME");
}

pub mod client {
    pub const DEFAULT_TIMEOUT: &str = "30s";
    pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    /// Number of entries to fetch in one request
    pub const DEFAULT_ENTRIES_PER_PAGINATION: i64 = 200;
    /// Number of feeds to fetch in one request
    pub const DEFAULT_FEEDS_PER_PAGINATION: i64 = 50;
}

pub mod credential {
    use std::time::Duration;
    /// The `Duration` considered as expired before actually performing the refresh process
    pub const EXPIRE_MARGIN: Duration = Duration::from_secs(60);
    pub const FALLBACK_EXPIRE: Duration = Duration::from_secs(60 * 15);
}

pub mod feed {
    /// Default entries limit to fetch
    pub const DEFAULT_ENTRIES_LIMIT: usize = 200;
}

pub mod cache {
    use std::path::Path;

    /// Credential cache file name
    pub const CREDENTIAL_FILE: &str = "credential.json";

    pub fn dir() -> &'static Path {
        super::project_dirs().cache_dir()
    }
}

pub(crate) mod github {
    /// GitHub pagination rest api is 1 origin
    pub(crate) const INITIAL_PAGE_NUM: u8 = 1;
    pub(crate) const NOTIFICATION_PER_PAGE: u8 = 40;
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
