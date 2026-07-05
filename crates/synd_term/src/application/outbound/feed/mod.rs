mod api;
mod client;

#[cfg(feature = "integration")]
mod mock;

pub use api::{FeedApi, FeedApiRef};
pub use client::ClientFeedApi;

#[cfg(feature = "integration")]
pub use mock::{MockFeedApi, MockFeedApiResponse};
