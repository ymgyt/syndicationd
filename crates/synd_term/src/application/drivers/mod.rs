use std::{
    num::NonZero,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{FutureExt as _, Stream};
use tokio::sync::mpsc;

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
    pub(super) cache: Cache,
    pub(super) clock: Box<dyn Clock>,
    feed: FeedDriver,
    auth: AuthDriver,
    gh: GhDriver,
    interaction: InteractionDriver,
    request_jobs: Jobs,
    next_request_id: u64,
    event_tx: mpsc::UnboundedSender<Event>,
    event_rx: mpsc::UnboundedReceiver<Event>,
}

pub(super) struct DriverParts {
    pub(super) feed_api: FeedApiRef,
    pub(super) gh_client: Option<GhClient>,
    pub(super) cache: Cache,
    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) clock: Option<Box<dyn Clock>>,
}

impl Drivers {
    pub(super) fn new(parts: DriverParts) -> Self {
        let DriverParts {
            feed_api,
            gh_client,
            cache,
            authenticator,
            interactor,
            clock,
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

        Self {
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
        }
    }

    /// Starts ordered external side effects requested by one state transition.
    pub(super) fn dispatch(&mut self, operations: Operations, terminal: &mut Terminal) {
        match operations {
            Operations::Nop => {}
            Operations::One(operation) => self.dispatch_operation(operation, terminal),
            Operations::Many(operations) => {
                for operation in operations {
                    self.dispatch_operation(operation, terminal);
                }
            }
        }
    }

    fn dispatch_operation(&mut self, operation: Operation, terminal: &mut Terminal) {
        match operation {
            Operation::StartDeviceFlow { provider } => {
                self.register_request(|drivers, cx| drivers.auth.start_device_flow(cx, provider));
            }
            Operation::PollDeviceFlowAccessToken {
                provider,
                device_authorization,
            } => {
                let now = self.clock.now();
                self.register_request(|drivers, cx| {
                    drivers.auth.poll_device_flow_access_token(
                        cx,
                        now,
                        provider,
                        *device_authorization,
                    )
                });
            }
            Operation::OpenFeedSubscriptionEditor => {
                let event = TerminalInteraction::new(&self.interaction, terminal)
                    .open_feed_subscription_editor();
                self.queue_event(event);
            }
            Operation::OpenFeedEditionEditor { prompt } => {
                let event = TerminalInteraction::new(&self.interaction, terminal)
                    .open_feed_edition_editor(prompt.as_str());
                self.queue_event(event);
            }
            Operation::SubscribeFeed { input } => {
                self.register_request(|drivers, cx| drivers.feed.subscribe_feed(cx, input));
            }
            Operation::UnsubscribeFeed { url } => {
                self.register_request(|drivers, cx| drivers.feed.unsubscribe_feed(cx, url));
            }
            Operation::FetchSubscription {
                generation,
                populate,
                after,
                first,
            } => {
                self.register_request(|drivers, cx| {
                    drivers
                        .feed
                        .fetch_subscription(cx, generation, populate, after, first)
                });
            }
            Operation::FetchTimelineWindow { limit } => {
                self.register_request(|drivers, cx| drivers.feed.fetch_timeline_window(cx, limit));
            }
            Operation::CatchUpTimeline { since } => {
                self.register_request(|drivers, cx| drivers.feed.catch_up_timeline(cx, since));
            }
            Operation::WatchFeedEvents => self.feed.watch_events(),
            Operation::FetchGhNotifications {
                generation,
                populate,
                params,
            } => {
                self.register_request(|drivers, cx| {
                    drivers
                        .gh
                        .fetch_notifications(cx, generation, populate, params)
                });
            }
            Operation::FetchGhDetail { generation, detail } => {
                self.register_request(|drivers, cx| {
                    drivers.gh.fetch_detail(cx, generation, detail)
                });
            }
            Operation::MarkGhNotificationAsDone { id } => {
                self.register_request(|drivers, cx| drivers.gh.mark_notification_as_done(cx, id));
            }
            Operation::UnsubscribeGhThread { id } => {
                self.register_request(|drivers, cx| drivers.gh.unsubscribe_thread(cx, id));
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
            Operation::ForceRedrawTerminal => terminal.force_redraw(),
            Operation::PersistCredential { credential } => {
                self.persist_credential(&credential);
            }
            Operation::SetCredential { credential } => self.set_credential(&credential),
        }
    }

    fn register_request<F>(&mut self, make_request: F)
    where
        F: FnOnce(&Self, RequestContext) -> (RequestKind, RequestFuture),
    {
        let request_id = self.assign_request_id();
        let context = RequestContext::new(request_id, self.event_tx.clone());
        let (kind, request) = make_request(self, context);
        let completion_tx = self.event_tx.clone();
        self.queue_event(Event::RequestEmitted {
            request_id,
            kind: kind.clone(),
        });
        let job: JobFuture = async move {
            let result = request.await;
            completion_tx
                .send(Event::RequestCompleted {
                    request_id,
                    kind,
                    result,
                })
                .expect("Drivers owns the event receiver");
        }
        .boxed();
        self.request_jobs.push(job);
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

    pub(super) fn shutdown(&mut self) {
        self.request_jobs.clear();
        self.auth.stop();
        self.feed.watcher.stop();
    }

    #[cfg(feature = "integration")]
    pub(super) fn request_jobs_is_empty(&self) -> bool {
        self.request_jobs.is_empty()
    }
}

impl Stream for Drivers {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Poll::Ready(Some(event)) = this.event_rx.poll_recv(cx) {
            return Poll::Ready(Some(event));
        }

        // Polling a job can enqueue its result.
        let _ = Pin::new(&mut this.request_jobs).poll_next(cx);

        if let Poll::Ready(Some(event)) = this.event_rx.poll_recv(cx) {
            return Poll::Ready(Some(event));
        }

        if let Poll::Ready(Some(event)) = Pin::new(&mut this.feed.watcher).poll_next(cx) {
            return Poll::Ready(Some(Event::Feeds(FeedsEvent::Push { event })));
        }

        if let Poll::Ready(Some(event)) = Pin::new(&mut this.auth).poll_next(cx) {
            return Poll::Ready(Some(event));
        }

        Poll::Pending
    }
}
