use crate::{
    client::synd_api::Client,
    local_api::{LocalApi, LocalApiHandle},
};

pub struct FeedBackend {
    client: Client,
    session: FeedApiSession,
    local_api_handle: Option<LocalApiHandle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedApiSession {
    UserCredentialRequired,
    Established,
}

impl FeedBackend {
    pub fn remote(client: Client) -> Self {
        Self {
            client,
            session: FeedApiSession::UserCredentialRequired,
            local_api_handle: None,
        }
    }

    pub fn local(local_api: LocalApi) -> Self {
        Self {
            client: local_api.client,
            session: FeedApiSession::Established,
            local_api_handle: Some(local_api.handle),
        }
    }

    pub(super) fn into_parts(self) -> (Client, FeedApiSession, Option<LocalApiHandle>) {
        (self.client, self.session, self.local_api_handle)
    }
}

impl FeedApiSession {
    pub(super) fn requires_user_credential(self) -> bool {
        matches!(self, Self::UserCredentialRequired)
    }
}
