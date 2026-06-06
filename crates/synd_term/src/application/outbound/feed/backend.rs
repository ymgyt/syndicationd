use std::sync::Arc;

use super::{ClientFeedApi, FeedApi, FeedApiRef};

/// Feed API implementation plus session state passed into `Application`.
pub struct FeedBackend {
    api: FeedApiRef,
    session: FeedApiSession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedApiSession {
    UserCredentialRequired,
    Established,
}

impl FeedBackend {
    pub fn new(api: FeedApiRef, session: FeedApiSession) -> Self {
        Self { api, session }
    }

    pub fn with_api(api: impl FeedApi, session: FeedApiSession) -> Self {
        Self::new(Arc::new(api), session)
    }

    pub fn from_client(client: synd_client::Client, session: FeedApiSession) -> Self {
        Self::with_api(ClientFeedApi::new(client), session)
    }

    pub fn established(client: synd_client::Client) -> Self {
        Self::from_client(client, FeedApiSession::Established)
    }

    pub(in crate::application) fn into_parts(self) -> (FeedApiRef, FeedApiSession) {
        (self.api, self.session)
    }
}

impl FeedApiSession {
    pub(in crate::application) fn requires_user_credential(self) -> bool {
        matches!(self, Self::UserCredentialRequired)
    }
}
