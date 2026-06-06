mod api;
mod backend;
mod client;

#[cfg(feature = "integration")]
mod mock;

pub use api::{FeedApi, FeedApiRef};
pub use backend::{FeedApiSession, FeedBackend};
pub use client::ClientFeedApi;

#[cfg(feature = "integration")]
pub use mock::{MockFeedApi, MockFeedApiResponse};
