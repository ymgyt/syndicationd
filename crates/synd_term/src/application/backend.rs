use synd_client::Client;

pub struct FeedBackend {
    client: Client,
    session: FeedApiSession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedApiSession {
    UserCredentialRequired,
    Established,
}

impl FeedBackend {
    pub fn new(client: Client, session: FeedApiSession) -> Self {
        Self { client, session }
    }

    pub fn established(client: Client) -> Self {
        Self {
            client,
            session: FeedApiSession::Established,
        }
    }

    pub(super) fn into_parts(self) -> (Client, FeedApiSession) {
        (self.client, self.session)
    }
}

impl FeedApiSession {
    pub(super) fn requires_user_credential(self) -> bool {
        matches!(self, Self::UserCredentialRequired)
    }
}
