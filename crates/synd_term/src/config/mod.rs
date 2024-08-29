use std::{path::PathBuf, sync::OnceLock};

use directories::ProjectDirs;

mod categories;
pub use categories::{Categories, Icon, IconColor};
mod file;
pub use file::INIT_CONFIG;
pub(crate) mod parse;

mod resolver;
pub use resolver::ConfigResolver;

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
    pub const CLIENT_TIMEOUT: &str = env_key!("CLIENT_TIMEOUT");
    pub const CONFIG_FILE: &str = env_key!("CONFIG_FILE");
    pub const LOG_FILE: &str = env_key!("LOG_FILE");
    pub const CACHE_DIR: &str = env_key!("CACHE_DIR");
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
    use std::path::PathBuf;

    /// Default entries limit to fetch
    pub const DEFAULT_ENTRIES_LIMIT: usize = 200;
    pub fn default_brower_command() -> PathBuf {
        PathBuf::new()
    }
}

pub mod cache {
    use std::path::Path;

    /// Credential cache file name
    pub const CREDENTIAL_FILE: &str = "credential.json";

    pub const GH_NOTIFICATION_FILTER_OPTION_FILE: &str = "gh_notification_filter_options.json";

    pub fn dir() -> &'static Path {
        super::project_dirs().cache_dir()
    }
}

pub(crate) mod github {
    /// GitHub pagination rest api is 1 origin
    pub(crate) const INITIAL_PAGE_NUM: u8 = 1;
    pub(crate) const NOTIFICATION_PER_PAGE: u8 = 40;
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
        // Prioritizing consistency with Linux, the qualifier and organization have not been specified
        ProjectDirs::from("", "", "syndicationd").expect("Failed to get project dirs")
    })
}
