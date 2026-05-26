use chrono::{DateTime, Utc};

use crate::{
    application::{Authenticator, Cache, Clock, FeedApiSession, SystemClock},
    client::{github::GithubClient, synd_api::Client},
    interact::Interact,
    local_api::LocalApiHandle,
    terminal::Terminal,
    ui::widgets::gh_notifications::GhNotificationFilterOptions,
};

pub(super) struct DriverAdapters {
    pub(super) clock: Box<dyn Clock>,
    pub(super) terminal: Terminal,
    pub(super) client: Client,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) github_client: Option<GithubClient>,
    pub(super) local_api_handle: Option<LocalApiHandle>,
    pub(super) cache: Cache,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) authenticator: Authenticator,
}

pub(super) struct DriverAdapterParts {
    pub(super) terminal: Terminal,
    pub(super) client: Client,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) github_client: Option<GithubClient>,
    pub(super) local_api_handle: Option<LocalApiHandle>,
    pub(super) cache: Cache,
    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) clock: Option<Box<dyn Clock>>,
}

impl DriverAdapters {
    pub(super) fn new(parts: DriverAdapterParts) -> Self {
        let DriverAdapterParts {
            terminal,
            client,
            feed_api_session,
            github_client,
            local_api_handle,
            cache,
            authenticator,
            interactor,
            clock,
        } = parts;

        Self {
            clock: clock.unwrap_or_else(|| Box::new(SystemClock)),
            terminal,
            client,
            feed_api_session,
            github_client,
            local_api_handle,
            cache,
            interactor,
            authenticator: authenticator.unwrap_or_else(Authenticator::new),
        }
    }

    pub(super) fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    pub(super) fn jwt_service(&self) -> &crate::application::JwtService {
        &self.authenticator.jwt_service
    }

    pub(super) fn persist_gh_notification_filter_options(
        &self,
        options: &GhNotificationFilterOptions,
    ) -> Result<(), crate::application::PersistCacheError> {
        self.cache.persist_gh_notification_filter_options(options)
    }
}
