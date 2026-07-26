#![warn(rustdoc::broken_intra_doc_links)]

mod client;
mod error;
pub mod payload;
mod scalar;

pub use client::{ApiCredential, Client, ClientOptions, FeedEventWatch};
pub use error::{Retryability, SyndApiError};
pub use scalar::{Category, FeedUrl, Rfc3339Time};
