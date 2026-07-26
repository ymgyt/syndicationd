use std::{
    future::Future,
    num::NonZero,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use chrono::{DateTime, Utc};
use futures_util::{FutureExt as _, Stream};
use ratatui::Frame;
use tokio::{
    sync::mpsc,
    time::{Instant, Sleep},
};

#[cfg(feature = "integration")]
use crate::auth::CredentialError;
use crate::{
    application::{
        Authenticator, Cache, Clock, FeedApiRef, RequestId, RequestKind, SystemClock,
        outbound::gh::GhClient,
    },
    auth::{Credential, Verified},
    event::{Event, FeedsEvent, OperationError},
    interact::Interact,
    job::Jobs,
    operation::{Operation, Operations},
    terminal::Terminal,
};

const REQUEST_JOB_CONCURRENCY: usize = 90;
const DISARMED_TIMER_DURATION: Duration = Duration::from_hours(24 * 365 * 30);

mod auth;
mod feed;
mod feed_event_watcher;
mod gh;
mod interaction;
mod request;

use auth::AuthDriver;
use feed::FeedDriver;
use gh::GhDriver;
use interaction::{InteractionDriver, TerminalInteraction};
use request::{JobFuture, RequestContext, RequestFuture};

/// Owns external ports, request execution, and long-lived event sources.
pub(super) struct Drivers {
    pub(super) terminal: Terminal,
    pub(super) cache: Cache,
    clock: Box<dyn Clock>,
    feed: FeedDriver,
    auth: AuthDriver,
    gh: GhDriver,
    interaction: InteractionDriver,
    request_jobs: Jobs,
    next_request_id: u64,
    event_tx: mpsc::UnboundedSender<Event>,
    event_rx: mpsc::UnboundedReceiver<Event>,
    throbber_timer: Pin<Box<Sleep>>,
    throbber_timer_interval: Duration,
    idle_timer: Pin<Box<Sleep>>,
}

pub(super) struct DriverParts {
    pub(super) terminal: Terminal,
    pub(super) feed_api: FeedApiRef,
    pub(super) gh_client: Option<GhClient>,
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
            gh_client,
            cache,
            authenticator,
            interactor,
            clock,
            throbber_timer_interval,
            idle_timer_interval,
        } = parts;
        let feed = {
            let api = feed_api.clone();
            FeedDriver::new(api)
        };
        let auth = {
            let authenticator = authenticator.unwrap_or_else(Authenticator::new);
            let api = feed_api;
            AuthDriver::new(authenticator, api)
        };
        let gh = GhDriver::new(gh_client);
        let interaction = InteractionDriver::new(interactor);
        let clock = clock.unwrap_or_else(|| Box::new(SystemClock));
        // The GitHub secondary rate limit is 100 concurrent requests.
        let request_jobs = Jobs::new(NonZero::new(REQUEST_JOB_CONCURRENCY).unwrap());
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let throbber_timer = Box::pin(tokio::time::sleep(DISARMED_TIMER_DURATION));
        let idle_timer = Box::pin(tokio::time::sleep(idle_timer_interval));

        Self {
            terminal,
            cache,
            clock,
            feed,
            auth,
            gh,
            interaction,
            request_jobs,
            next_request_id: 0,
            event_tx,
            event_rx,
            throbber_timer,
            throbber_timer_interval,
            idle_timer,
        }
    }

    /// Starts ordered external side effects requested by one state transition.
    pub(super) fn dispatch(&mut self, operations: Operations) {
        match operations {
            Operations::Nop => {}
            Operations::One(operation) => self.dispatch_operation(operation),
            Operations::Many(operations) => {
                for operation in operations {
                    self.dispatch_operation(operation);
                }
            }
        }
    }

    fn dispatch_operation(&mut self, operation: Operation) {
        match operation {
            Operation::StartDeviceFlow { provider } => {
                let make_request = self.auth.start_device_flow(provider);
                self.register_request(RequestKind::StartDeviceFlow { provider }, make_request);
            }
            Operation::PollDeviceFlowAccessToken {
                provider,
                device_authorization,
            } => {
                let now = self.clock.now();
                let make_request =
                    self.auth
                        .poll_device_flow_access_token(now, provider, *device_authorization);
                self.register_request(
                    RequestKind::PollDeviceFlowAccessToken { provider },
                    make_request,
                );
            }
            Operation::OpenFeedSubscriptionEditor => {
                let event = TerminalInteraction::new(&self.interaction, &mut self.terminal)
                    .open_feed_subscription_editor();
                self.queue_event(event);
            }
            Operation::OpenFeedEditionEditor { prompt } => {
                let event = TerminalInteraction::new(&self.interaction, &mut self.terminal)
                    .open_feed_edition_editor(prompt.as_str());
                self.queue_event(event);
            }
            Operation::SubscribeFeed { input } => {
                let kind = RequestKind::SubscribeFeed {
                    url: input.url.clone(),
                };
                let make_request = self.feed.subscribe_feed(input);
                self.register_request(kind, make_request);
            }
            Operation::UnsubscribeFeed { url } => {
                let kind = RequestKind::UnsubscribeFeed { url: url.clone() };
                let make_request = self.feed.unsubscribe_feed(url);
                self.register_request(kind, make_request);
            }
            Operation::FetchSubscription {
                populate,
                after,
                first,
            } => {
                let make_request = self.feed.fetch_subscription(populate, after, first);
                self.register_request(RequestKind::FetchSubscription, make_request);
            }
            Operation::FetchTimelineWindow { limit } => {
                let make_request = self.feed.fetch_timeline_window(limit);
                self.register_request(RequestKind::FetchTimelineWindow { limit }, make_request);
            }
            Operation::CatchUpTimeline { since } => {
                let make_request = self.feed.catch_up_timeline(since);
                self.register_request(RequestKind::CatchUpTimeline { since }, make_request);
            }
            Operation::WatchFeedEvents => self.feed.watch_events(),
            Operation::FetchGhNotifications { populate, params } => {
                let kind = RequestKind::FetchGhNotifications { page: params.page };
                let make_request = self.gh.fetch_notifications(populate, params);
                self.register_request(kind, make_request);
            }
            Operation::FetchGhIssue { context } => {
                let kind = RequestKind::FetchGhIssue { id: context.id };
                let make_request = self.gh.fetch_issue(context);
                self.register_request(kind, make_request);
            }
            Operation::FetchGhPullRequest { context } => {
                let kind = RequestKind::FetchGhPullRequest { id: context.id };
                let make_request = self.gh.fetch_pull_request(context);
                self.register_request(kind, make_request);
            }
            Operation::MarkGhNotificationAsDone { id } => {
                let make_request = self.gh.mark_notification_as_done(id);
                self.register_request(RequestKind::MarkGhNotificationAsDone { id }, make_request);
            }
            Operation::UnsubscribeGhThread { id } => {
                let make_request = self.gh.unsubscribe_thread(id);
                self.register_request(RequestKind::UnsubscribeGhThread { id }, make_request);
            }
            Operation::OpenBrowser { url } => {
                if let Some(event) = self.interaction.open_browser(url) {
                    self.queue_event(event);
                }
            }
            Operation::OpenTextBrowser { url } => {
                if let Some(event) = self.interaction.open_text_browser(url) {
                    self.queue_event(event);
                }
            }
            Operation::ForceRedrawTerminal => self.terminal.force_redraw(),
            Operation::PersistCredential { credential } => {
                self.persist_credential(&credential);
            }
            Operation::SetCredential { credential } => self.set_credential(&credential),
        }
    }

    fn register_request<F>(&mut self, kind: RequestKind, make_request: F)
    where
        F: FnOnce(RequestContext) -> RequestFuture,
    {
        let request_id = self.assign_request_id();
        let context = RequestContext::new(request_id, self.event_tx.clone());
        let request = make_request(context);
        let completion_tx = self.event_tx.clone();
        let job: JobFuture = async move {
            let result = request.await;
            completion_tx
                .send(Event::RequestCompleted { request_id, result })
                .expect("Drivers owns the event receiver");
        }
        .boxed();
        let should_arm_throbber = self.request_jobs.is_empty();

        self.request_jobs.push(job);
        if should_arm_throbber {
            self.reset_throbber_timer();
        }
        self.queue_event(Event::RequestEmitted { request_id, kind });
    }

    fn persist_credential(&self, credential: &Verified<Credential>) {
        if let Err(error) = self.cache.persist_credential(credential) {
            self.queue_event(Event::OperationFailed {
                error: OperationError::PersistCredential(error),
            });
        }
    }

    fn assign_request_id(&mut self) -> RequestId {
        let request_id = RequestId::new(self.next_request_id);
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .expect("request ID sequence exhausted");
        request_id
    }

    fn queue_event(&self, event: Event) {
        self.event_tx
            .send(event)
            .expect("Drivers owns the event receiver");
    }

    fn set_credential(&mut self, credential: &Verified<Credential>) {
        match self.auth.set_credential(self.clock.now(), credential) {
            Ok(()) => {
                self.feed.restart_events_if_started();
                self.queue_event(Event::ApiCredentialConfigured);
            }
            Err(error) => self.queue_event(Event::OperationFailed {
                error: OperationError::SetCredential(error),
            }),
        }
    }

    fn reset_throbber_timer(&mut self) {
        self.throbber_timer
            .as_mut()
            .reset(Instant::now() + self.throbber_timer_interval);
    }

    fn disarm_throbber_timer(&mut self) {
        Self::disarm_timer(self.throbber_timer.as_mut());
    }

    fn disarm_timer(mut timer: Pin<&mut Sleep>) {
        timer
            .as_mut()
            .reset(Instant::now() + DISARMED_TIMER_DURATION);
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

    pub(super) fn render<F>(&mut self, render: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Frame, DateTime<Utc>),
    {
        let now = self.clock.now();
        self.terminal.render(|frame| render(frame, now))
    }

    pub(super) fn clear_idle_timer(&mut self) {
        Self::disarm_timer(self.idle_timer.as_mut());
    }

    pub(super) fn reset_idle_timer(&mut self, interval: Duration) {
        self.idle_timer.as_mut().reset(Instant::now() + interval);
    }

    pub(super) fn shutdown(&mut self) {
        self.request_jobs.clear();
        self.auth.stop();
        self.feed.watcher.stop();
        self.disarm_throbber_timer();
        self.clear_idle_timer();
    }

    #[cfg(feature = "integration")]
    pub(super) fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.buffer()
    }

    #[cfg(feature = "integration")]
    pub(super) fn request_jobs_is_empty(&self) -> bool {
        self.request_jobs.is_empty()
    }
}

/// Polls driver-owned event sources in this priority order: queued events,
/// request jobs, feed events, authentication, throbber, then idle.
///
/// A request job yields `()` but can enqueue events while it is polled. The
/// second `event_rx` poll exposes those events before lower-priority sources.
/// Polling every remaining source gives it a chance to register `cx.waker()`
/// before returning `Pending`.
impl Stream for Drivers {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Poll::Ready(Some(event)) = this.event_rx.poll_recv(cx) {
            return Poll::Ready(Some(event));
        }

        if let Poll::Ready(Some(())) = Pin::new(&mut this.request_jobs).poll_next(cx)
            && this.request_jobs.is_empty()
        {
            this.disarm_throbber_timer();
        }

        if let Poll::Ready(Some(event)) = this.event_rx.poll_recv(cx) {
            return Poll::Ready(Some(event));
        }

        if let Poll::Ready(Some(event)) = Pin::new(&mut this.feed.watcher).poll_next(cx) {
            return Poll::Ready(Some(Event::Feeds(FeedsEvent::Push { event })));
        }

        if let Poll::Ready(Some(event)) = Pin::new(&mut this.auth).poll_next(cx) {
            return Poll::Ready(Some(event));
        }

        if !this.request_jobs.is_empty() && this.throbber_timer.as_mut().poll(cx).is_ready() {
            this.reset_throbber_timer();
            return Poll::Ready(Some(Event::ThrobberTick));
        }

        if this.idle_timer.as_mut().poll(cx).is_ready() {
            Self::disarm_timer(this.idle_timer.as_mut());
            return Poll::Ready(Some(Event::Idle));
        }

        Poll::Pending
    }
}
