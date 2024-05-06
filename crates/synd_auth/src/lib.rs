//! syndicationd authentication crate providing features
//! related OAuth and JWT.
#![warn(rustdoc::broken_intra_doc_links)]

mod config;
pub mod device_flow;
pub mod jwt;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
