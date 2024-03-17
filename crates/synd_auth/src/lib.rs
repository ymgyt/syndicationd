pub mod config;
pub mod device_flow;
pub mod jwt;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
