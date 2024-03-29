pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub const PORT: u16 = 5959;

pub mod env {
    macro_rules! env_key {
        ($key:expr) => {
            concat!("SYND", "_", $key)
        };
    }
    pub(crate) use env_key;
}

pub mod serve {
    pub const DEFAULT_ADDR: &str = "127.0.0.1";
    pub const DEFAULT_PORT: u16 = 5959;
    pub const DEFAULT_REQUEST_TIMEOUT: &str = "30s";
    pub const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1024 * 2;
    pub const DEFAULT_REQUEST_CONCURRENCY_LIMIT: usize = 100;

    pub const HEALTH_CHECK_PATH: &str = "/health";
}
}
