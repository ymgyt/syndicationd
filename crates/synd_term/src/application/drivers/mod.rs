use std::time::Duration;

use chrono::{DateTime, Utc};
use ratatui::Frame;

#[cfg(feature = "integration")]
use crate::auth::CredentialError;
use crate::{
    application::outbound::github::GithubClient,
    application::{Authenticator, Cache, Clock, FeedApiRef, SystemClock},
    auth::{Credential, Verified},
    event::Event,
    interact::Interact,
    operation::Operation,
    terminal::Terminal,
};

use super::TIMELINE_INVALIDATION_DEBOUNCE;

mod auth;
mod feed;
mod feed_events;
mod github;
mod interaction;
mod runtime;

use auth::AuthDriver;
use feed::FeedDriver;
use feed_events::FeedEventSubscription;
use github::GitHubDriver;
use interaction::InteractionDriver;
use runtime::{DriverPollers, DriverRuntime};

/// Executes side effects requested as `Operation`, owning the external-world
/// handles and the machinery(job queues, timers, in-flight tracking) that
/// runs them.
pub(super) struct Drivers {
    pub(super) terminal: Terminal,
    pub(super) cache: Cache,
    clock: Box<dyn Clock>,
    feed: FeedDriver,
    auth: AuthDriver,
    github: GitHubDriver,
    interaction: InteractionDriver,
    feed_events: FeedEventSubscription,
    runtime: DriverRuntime,
}

pub(super) struct DriverParts {
    pub(super) terminal: Terminal,
    pub(super) feed_api: FeedApiRef,
    pub(super) github_client: Option<GithubClient>,
    pub(super) cache: Cache,
    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) clock: Option<Box<dyn Clock>>,
    pub(super) throbber_timer_interval: Duration,
    pub(super) idle_timer_interval: Duration,
}

impl Drivers {
    pub(super) fn new(parts: DriverParts) -> Self {
        let DriverParts {
            terminal,
            feed_api,
            github_client,
            cache,
            authenticator,
            interactor,
            clock,
            throbber_timer_interval,
            idle_timer_interval,
        } = parts;

        Self {
            terminal,
            cache,
            clock: clock.unwrap_or_else(|| Box::new(SystemClock)),
            feed: FeedDriver {
                api: feed_api.clone(),
            },
            auth: AuthDriver {
                authenticator: authenticator.unwrap_or_else(Authenticator::new),
                api: feed_api,
            },
            github: GitHubDriver {
                client: github_client,
            },
            interaction: InteractionDriver { interactor },
            feed_events: FeedEventSubscription::new(),
            runtime: DriverRuntime::new(throbber_timer_interval, idle_timer_interval),
        }
    }

    /// Route an `Operation` to the driver that executes it.
    pub(super) fn dispatch(&mut self, operation: Operation) -> Vec<Event> {
        match operation {
            Operation::StartDeviceFlow { provider } => {
                self.auth.start_device_flow(&mut self.runtime, provider);
                Vec::new()
            }
            Operation::PollDeviceFlowAccessToken {
                provider,
                device_authorization,
            } => {
                self.auth.poll_device_flow_access_token(
                    &mut self.runtime,
                    self.clock.now(),
                    provider,
                    *device_authorization,
                );
                Vec::new()
            }
            Operation::OpenFeedSubscriptionEditor => vec![
                self.interaction
                    .open_feed_subscription_editor(&mut self.terminal),
            ],
            Operation::OpenFeedEditionEditor { prompt } => vec![
                self.interaction
                    .open_feed_edition_editor(&mut self.terminal, prompt.as_str()),
            ],
            Operation::SubscribeFeed { input } => {
                self.feed.subscribe_feed(&mut self.runtime, input);
                Vec::new()
            }
            Operation::FetchSubscription {
                populate,
                after,
                first,
            } => {
                self.feed
                    .fetch_subscription(&mut self.runtime, populate, after, first);
                Vec::new()
            }
            Operation::FetchEntries {
                populate,
                after,
                first,
            } => self
                .feed
                .fetch_entries(&mut self.runtime, populate, after, first)
                .into_iter()
                .collect(),
            Operation::SyncTimeline { since } => {
                self.feed.sync_timeline(&mut self.runtime, since);
                Vec::new()
            }
            Operation::StartFeedEventSubscription => {
                self.feed_events.start(self.feed.api.clone());
                Vec::new()
            }
            Operation::UnsubscribeFeed { url } => {
                self.feed.unsubscribe_feed(&mut self.runtime, url);
                Vec::new()
            }
            Operation::ScheduleTimelineSync => {
                self.runtime
                    .schedule_event(TIMELINE_INVALIDATION_DEBOUNCE, Event::TimelineSyncDebounced);
                Vec::new()
            }
            Operation::FetchGitHubNotifications { populate, params } => self
                .github
                .fetch_notifications(&mut self.runtime, populate, params)
                .into_iter()
                .collect(),
            Operation::FetchGitHubNotificationDetails { contexts } => self
                .github
                .fetch_notification_details(&mut self.runtime, contexts)
                .into_iter()
                .collect(),
            Operation::MarkGitHubNotificationAsDone { id } => self
                .github
                .mark_notification_as_done(&mut self.runtime, id)
                .into_iter()
                .collect(),
            Operation::UnsubscribeGitHubThread { id } => self
                .github
                .unsubscribe_thread(&mut self.runtime, id)
                .into_iter()
                .collect(),
            Operation::OpenBrowser { url } => {
                self.interaction.open_browser(url).into_iter().collect()
            }
            Operation::OpenTextBrowser { url } => self
                .interaction
                .open_text_browser(url)
                .into_iter()
                .collect(),
            Operation::ForceRedrawTerminal => {
                self.terminal.force_redraw();
                Vec::new()
            }
        }
    }

    pub(super) fn pollers(&mut self) -> DriverPollers<'_> {
        DriverPollers {
            jobs: &mut self.runtime.jobs,
            background_jobs: &mut self.runtime.background_jobs,
            feed_events: &mut self.feed_events,
            in_flight: &mut self.runtime.in_flight,
            idle_timer: &mut self.runtime.idle_timer,
        }
    }

    pub(super) fn reset_throbber(&mut self) {
        self.runtime.reset_throbber();
    }

    pub(super) fn remove_in_flight(
        &mut self,
        request_seq: super::RequestSequence,
    ) -> Option<super::RequestId> {
        self.runtime.remove_in_flight(request_seq)
    }

    pub(super) fn has_in_flight(&self, request_id: super::RequestId) -> bool {
        self.runtime.has_in_flight(request_id)
    }

    pub(super) fn set_credential(&mut self, cred: Verified<Credential>) {
        let now = self.clock.now();
        self.auth.set_credential(&mut self.runtime, now, cred);
    }

    #[cfg(feature = "integration")]
    pub(super) async fn restore_credential(&self) -> Result<Verified<Credential>, CredentialError> {
        let restore = crate::auth::Restore {
            jwt_service: &self.auth.authenticator.jwt_service,
            cache: &self.cache,
            now: self.clock.now(),
            persist_when_refreshed: true,
        };
        restore.restore().await
    }

    pub(super) fn render<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Frame, &super::InFlight, DateTime<Utc>),
    {
        let in_flight = &self.runtime.in_flight;
        let now = self.clock.now();
        self.terminal.render(|frame| f(frame, in_flight, now))
    }

    pub(super) fn restart_feed_events_if_running(&mut self) -> bool {
        self.feed_events.restart_if_running(self.feed.api.clone())
    }

    pub(super) fn clear_idle_timer(&mut self) {
        self.runtime.clear_idle_timer();
    }

    pub(super) fn reset_idle_timer(&mut self, interval: Duration) {
        self.runtime.reset_idle_timer(interval);
    }

    pub(super) fn shutdown(&mut self) {
        self.feed_events.stop();
    }

    #[cfg(feature = "integration")]
    pub(super) fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.buffer()
    }

    #[cfg(feature = "integration")]
    pub(super) fn foreground_jobs_is_empty(&self) -> bool {
        self.runtime.jobs.is_empty()
    }
}
