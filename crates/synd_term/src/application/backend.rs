use crate::{
    client::synd_api::Client,
    local_api::{LocalApi, LocalApiRuntime},
};

pub struct FeedBackend {
    client: Client,
    session: FeedApiSession,
    local_api_runtime: Option<LocalApiRuntime>,
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
            local_api_runtime: None,
        }
    }

    pub fn local(local_api: LocalApi) -> Self {
        Self {
            client: local_api.client,
            session: FeedApiSession::Established,
            local_api_runtime: Some(local_api.runtime),
        }
    }

    pub(super) fn into_parts(self) -> (Client, FeedApiSession, Option<LocalApiRuntime>) {
        (self.client, self.session, self.local_api_runtime)
    }
}

impl FeedApiSession {
    pub(super) fn requires_user_credential(self) -> bool {
        matches!(self, Self::UserCredentialRequired)
    }
}
