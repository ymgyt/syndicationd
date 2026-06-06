use std::time::Duration;

use chrono::{DateTime, Utc};
use ratatui::Frame;

use crate::{
    application::outbound::github::GithubClient,
    application::{
        Authenticator, Cache, Clock, FeedApiRef, FeedApiSession, LoadCacheError, PersistCacheError,
    },
    auth::{Credential, CredentialError, Verified},
    event::Event,
    interact::Interact,
    operation::Operation,
    terminal::Terminal,
    ui::widgets::gh_notifications::GhNotificationFilterOptions,
};

mod adapters;
mod auth;
mod dispatcher;
mod feed;
mod github;
mod interaction;
mod runtime;
mod timeline;

use adapters::DriverAdapters;
use dispatcher::OperationDispatcher;
use runtime::{DriverPollers, DriverRuntime};
use timeline::TimelineChangeSubscription;

/// Facade over outside-world adapters, execution machinery, and operation routing.
pub(super) struct Drivers {
    adapters: DriverAdapters,
    runtime: DriverRuntime,
    timeline_changes: TimelineChangeSubscription,
    operation_dispatcher: OperationDispatcher,
}

pub(super) struct DriverParts {
    pub(super) terminal: Terminal,
    pub(super) feed_api: FeedApiRef,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) github_client: Option<GithubClient>,
    pub(super) cache: Cache,
    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) clock: Option<Box<dyn Clock>>,
    pub(super) throbber_timer_interval: Duration,
    pub(super) idle_timer_interval: Duration,
}

pub(super) struct DriverContext<'a> {
    adapters: &'a mut DriverAdapters,
    runtime: &'a mut DriverRuntime,
    timeline_changes: &'a mut TimelineChangeSubscription,
}

impl Drivers {
    pub(super) fn new(parts: DriverParts) -> Self {
        let DriverParts {
            terminal,
            feed_api,
            feed_api_session,
            github_client,
            cache,
            authenticator,
            interactor,
            clock,
            throbber_timer_interval,
            idle_timer_interval,
        } = parts;

        Self {
            adapters: DriverAdapters::new(adapters::DriverAdapterParts {
                terminal,
                feed_api,
                feed_api_session,
                github_client,
                cache,
                authenticator,
                interactor,
                clock,
            }),
            runtime: DriverRuntime::new(throbber_timer_interval, idle_timer_interval),
            timeline_changes: TimelineChangeSubscription::new(),
            operation_dispatcher: OperationDispatcher::new(),
        }
    }

    pub(super) fn perform_operation(&mut self, operation: Operation) -> Vec<Event> {
        let dispatcher = self.operation_dispatcher;
        let mut cx = self.context();
        dispatcher.dispatch(operation, &mut cx)
    }

    pub(super) fn pollers(&mut self) -> DriverPollers<'_> {
        DriverPollers {
            jobs: &mut self.runtime.jobs,
            background_jobs: &mut self.runtime.background_jobs,
            timeline_changes: &mut self.timeline_changes,
            in_flight: &mut self.runtime.in_flight,
            idle_timer: &mut self.runtime.idle_timer,
        }
    }

    pub(super) fn reset_throbber(&mut self) {
        self.runtime.reset_throbber();
    }

    pub(super) fn remove_in_flight(&mut self, request_seq: super::RequestSequence) {
        self.runtime.remove_in_flight(request_seq);
    }

    pub(super) fn set_credential(&mut self, cred: Verified<Credential>) {
        let mut cx = self.context();
        auth::AuthDriver::set_credential(&mut cx, cred);
    }

    pub(super) fn persist_credential(
        &self,
        cred: &Verified<Credential>,
    ) -> Result<(), PersistCacheError> {
        self.adapters.cache.persist_credential(cred)
    }

    pub(super) async fn restore_credential(&self) -> Result<Verified<Credential>, CredentialError> {
        let restore = crate::auth::Restore {
            jwt_service: self.adapters.jwt_service(),
            cache: &self.adapters.cache,
            now: self.adapters.now(),
            persist_when_refreshed: true,
        };
        restore.restore().await
    }

    pub(super) fn init_terminal(&mut self) -> std::io::Result<()> {
        self.adapters.terminal.init()
    }

    pub(super) fn restore_terminal(&mut self) -> std::io::Result<()> {
        self.adapters.terminal.restore()
    }

    pub(super) fn render<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Frame, &super::InFlight, DateTime<Utc>),
    {
        let in_flight = &self.runtime.in_flight;
        let now = self.adapters.now();
        self.adapters
            .terminal
            .render(|frame| f(frame, in_flight, now))
    }

    pub(super) fn feed_api_session_requires_user_credential(&self) -> bool {
        self.adapters.feed_api_session.requires_user_credential()
    }

    pub(super) fn supports_timeline_change_subscription(&self) -> bool {
        self.adapters
            .feed_api
            .supports_timeline_change_subscription()
    }

    pub(super) fn restart_timeline_changes_if_running(&mut self) -> bool {
        let feed_api = self.adapters.feed_api.clone();
        self.timeline_changes.restart_if_running(feed_api)
    }

    pub(super) fn load_gh_notification_filter_options(
        &self,
    ) -> Result<GhNotificationFilterOptions, LoadCacheError> {
        self.adapters.cache.load_gh_notification_filter_options()
    }

    pub(super) fn persist_gh_notification_filter_options(
        &self,
        options: &GhNotificationFilterOptions,
    ) -> Result<(), PersistCacheError> {
        self.adapters
            .persist_gh_notification_filter_options(options)
    }

    pub(super) fn clean_cache(&self) {
        self.adapters.cache.clean_credential().ok();
    }

    pub(super) fn clear_idle_timer(&mut self) {
        self.runtime.clear_idle_timer();
    }

    pub(super) fn reset_idle_timer(&mut self, interval: Duration) {
        self.runtime.reset_idle_timer(interval);
    }

    pub(super) fn shutdown(&mut self) {
        self.timeline_changes.stop();
    }

    #[cfg(feature = "integration")]
    pub(super) fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.adapters.terminal.buffer()
    }

    #[cfg(feature = "integration")]
    pub(super) fn foreground_jobs_is_empty(&self) -> bool {
        self.runtime.jobs.is_empty()
    }

    fn context(&mut self) -> DriverContext<'_> {
        DriverContext {
            adapters: &mut self.adapters,
            runtime: &mut self.runtime,
            timeline_changes: &mut self.timeline_changes,
        }
    }
}
