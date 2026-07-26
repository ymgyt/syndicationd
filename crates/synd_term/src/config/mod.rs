mod categories;
pub use categories::{Categories, CategoryConfig, Icon, IconColor};

pub mod client {
    use std::time::Duration;

    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
    pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    /// Number of feeds to fetch in one request
    pub const DEFAULT_FEEDS_PER_PAGINATION: i64 = 50;
}

pub mod credential {
    use std::time::Duration;
    /// The `Duration` considered as expired before actually performing the refresh process
    pub const EXPIRE_MARGIN: Duration = Duration::from_mins(1);
    pub const FALLBACK_EXPIRE: Duration = Duration::from_mins(15);
}

pub mod feed {
    use std::path::PathBuf;

    /// Default entries limit to fetch
    pub const DEFAULT_ENTRIES_LIMIT: usize = 200;
    pub fn default_browser_command() -> PathBuf {
        PathBuf::new()
    }
}

pub mod cache {
    /// Credential cache file name
    pub const CREDENTIAL_FILE: &str = "credential.json";

    pub const GH_NOTIFICATION_FILTER_OPTION_FILE: &str = "gh_notification_filter_options.json";
}

pub(crate) mod gh {
    use std::time::Duration;

    /// GitHub pagination rest api is 1 origin
    pub(crate) const INITIAL_PAGE_NUM: u8 = 1;
    pub(crate) const NOTIFICATION_PER_PAGE: u8 = 40;
    pub(crate) const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);
}
