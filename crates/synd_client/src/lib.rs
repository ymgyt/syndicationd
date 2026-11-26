#![warn(rustdoc::broken_intra_doc_links)]

mod client;
mod error;
pub mod payload;
mod scalar;

pub use client::{ApiCredential, Client, ClientOptions, FeedEventWatch};
pub use error::{Retryability, SyndApiError};
pub use scalar::{FeedUrl, Rfc3339Time};
pub use synd_feed::types::SubscriptionLabels;
