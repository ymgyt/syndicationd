use std::path::PathBuf;

use synd_support::dirs::SyndicationdDirs;

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
    pub const DAEMON_SESSION_LEASE_DURATION: &str = env_key!("DAEMON_SESSION_LEASE_DURATION");
    pub const DAEMON_SESSION_IDLE_SHUTDOWN_GRACE: &str =
        env_key!("DAEMON_SESSION_IDLE_SHUTDOWN_GRACE");
}

pub mod client {
    use std::time::Duration;

    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
}

pub mod daemon {
    use std::time::Duration;

    pub fn default_session_lease_duration() -> Duration {
        synd_runtime::DaemonSessionConfig::default()
            .lease_policy()
            .lease_duration()
    }

    pub fn default_session_idle_shutdown_grace() -> Duration {
        synd_runtime::DaemonSessionConfig::default().idle_shutdown_grace()
    }
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
        super::dirs().cache_dir()
    }
}

pub mod local {
    use std::path::PathBuf;

    pub fn sqlite_db() -> PathBuf {
        super::dirs().sqlite_db()
    }
}

pub(crate) mod theme {
    use crate::cli::Palette;

    pub(crate) const DEFAULT_PALETTE: Palette = Palette::Ferra;
}

pub fn log_path() -> PathBuf {
    dirs().log_file()
}

pub fn config_path() -> PathBuf {
    dirs().config_file()
}

fn dirs() -> &'static SyndicationdDirs {
    SyndicationdDirs::current()
}
