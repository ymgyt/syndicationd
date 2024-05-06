//! syndicationd graphql api server crate
#![allow(clippy::new_without_default)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod args;
pub mod client;
pub mod config;
pub mod dependency;
pub(crate) mod gql;
pub mod monitor;
pub(crate) mod principal;
pub mod repository;
pub mod serve;
pub mod shutdown;
pub mod usecase;
