mod client;
pub use client::{GithubClient, GithubClientError};
#[path = "generated/query.rs"]
pub mod query;
