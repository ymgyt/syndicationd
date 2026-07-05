use chrono::{DateTime, Utc};

use crate::{
    application::outbound::github::GithubClient,
    application::{Authenticator, Cache, Clock, FeedApiRef, FeedApiSession, SystemClock},
    interact::Interact,
    terminal::Terminal,
    ui::widgets::gh_notifications::GhNotificationFilterOptions,
};

pub(in crate::application) struct DriverHandles {
    pub(super) clock: Box<dyn Clock>,
    pub(in crate::application) terminal: Terminal,
    pub(super) feed_api: FeedApiRef,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) github_client: Option<GithubClient>,
    pub(super) cache: Cache,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) authenticator: Authenticator,
}

pub(super) struct DriverHandleParts {
    pub(super) terminal: Terminal,
    pub(super) feed_api: FeedApiRef,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) github_client: Option<GithubClient>,
    pub(super) cache: Cache,
    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) clock: Option<Box<dyn Clock>>,
}

impl DriverHandles {
    pub(super) fn new(parts: DriverHandleParts) -> Self {
        let DriverHandleParts {
            terminal,
            feed_api,
            feed_api_session,
            github_client,
            cache,
            authenticator,
            interactor,
            clock,
        } = parts;

        Self {
            clock: clock.unwrap_or_else(|| Box::new(SystemClock)),
            terminal,
            feed_api,
            feed_api_session,
            github_client,
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
