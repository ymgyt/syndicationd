use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::Stream;
use synd_support::html::HtmlConverter;
use tuiframework::{Driver, DriverContext, driver::DriverMessage};

#[cfg(feature = "integration")]
use crate::auth::CredentialError;
use crate::{
    application::{Authenticator, Cache, Clock, FeedApiRef, RequestKind, outbound::gh::GhClient},
    auth::{Credential, Verified},
    event::{Event, FeedsEvent, OperationError},
    interact::Interact,
    operation::Operation,
};

mod auth;
mod feed;
mod feed_event_watcher;
mod gh;
mod interaction;
mod reader;
mod request;

use auth::AuthDriver;
use feed::FeedDriver;
use gh::GhDriver;
use interaction::InteractionDriver;
use reader::ReaderDriver;
use request::{RequestContext, RequestFuture};

/// Owns external ports, request execution, and long-lived event sources.
pub(super) struct Drivers {
    pub(super) cache: Cache,
    pub(super) clock: Arc<dyn Clock>,
    feed: FeedDriver,
    auth: AuthDriver,
    gh: GhDriver,
    interaction: InteractionDriver,
    reader: ReaderDriver,
}

pub(super) struct DriverParts {
    pub(super) feed_api: FeedApiRef,
    pub(super) gh_client: Option<GhClient>,
    pub(super) cache: Cache,
    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Box<dyn Interact>,
    pub(super) clock: Arc<dyn Clock>,
    pub(super) html_converter: Arc<dyn HtmlConverter>,
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
            html_converter,
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

        Self {
            cache,
            clock,
            feed,
            auth,
            gh,
            interaction,
            reader: ReaderDriver::new(html_converter),
        }
    }
}

impl Driver for Drivers {
    type Operation = Operation;
    type Event = Event;

    fn shutdown(&mut self) {
        self.auth.stop();
        self.feed.watcher.stop();
    }
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<DriverMessage<Event, Operation>> {
        if let Poll::Ready(Some(event)) = Pin::new(&mut self.feed.watcher).poll_next(cx) {
            return Poll::Ready(DriverMessage::Event(Event::Feeds(FeedsEvent::Push {
                event,
            })));
        }
        if let Poll::Ready(Some(event)) = Pin::new(&mut self.auth).poll_next(cx) {
            return Poll::Ready(DriverMessage::Event(event));
        }
        Poll::Pending
    }
    fn dispatch(&mut self, operation: Operation, context: &mut DriverContext<Event>) {
        match operation {
            Operation::StartDeviceFlow { provider } => {
                Self::register_request(context, |cx| self.auth.start_device_flow(cx, provider));
            }
            Operation::PollDeviceFlowAccessToken {
                provider,
                device_authorization,
            } => {
                let now = self.clock.now();
                Self::register_request(context, |cx| {
                    self.auth.poll_device_flow_access_token(
                        cx,
                        now,
                        provider,
                        *device_authorization,
                    )
                });
            }
            Operation::OpenFeedSubscriptionEditor => {
                self.interaction.open_feed_subscription_editor(context);
            }
            Operation::OpenFeedEditionEditor { prompt } => {
                self.interaction.open_feed_edition_editor(&prompt, context);
            }
            Operation::SubscribeFeed { input } => {
                Self::register_request(context, |cx| self.feed.subscribe_feed(cx, input));
            }
            Operation::UnsubscribeFeed { url } => {
                Self::register_request(context, |cx| self.feed.unsubscribe_feed(cx, url));
            }
            Operation::FetchSubscription {
                generation,
                populate,
                after,
                first,
            } => {
                Self::register_request(context, |cx| {
                    self.feed
                        .fetch_subscription(cx, generation, populate, after, first)
                });
            }
            Operation::OpenInitialTimeline => {
                Self::register_request(context, |cx| self.feed.open_initial_timeline(cx));
            }
            Operation::FetchTimelineWindow { timeline_id, limit } => {
                Self::register_request(context, |cx| {
                    self.feed.fetch_timeline_window(cx, timeline_id, limit)
                });
            }
            Operation::CatchUpTimeline { timeline_id, since } => {
                Self::register_request(context, |cx| {
                    self.feed.catch_up_timeline(cx, timeline_id, since)
                });
            }
            Operation::WatchFeedEvents => self.feed.watch_events(),
            Operation::FetchGhNotifications {
                generation,
                populate,
                params,
            } => {
                Self::register_request(context, |cx| {
                    self.gh
                        .fetch_notifications(cx, generation, populate, params)
                });
            }
            Operation::FetchGhDetail { generation, detail } => {
                Self::register_request(context, |cx| self.gh.fetch_detail(cx, generation, detail));
            }
            Operation::MarkGhNotificationAsDone { id } => {
                Self::register_request(context, |cx| self.gh.mark_notification_as_done(cx, id));
            }
            Operation::UnsubscribeGhThread { id } => {
                Self::register_request(context, |cx| self.gh.unsubscribe_thread(cx, id));
            }
            Operation::OpenBrowser { url } => {
                if let Some(event) = self.interaction.open_browser(url) {
                    context.emit(event);
                }
            }
            Operation::FetchArticle { generation, url } => {
                Self::register_request(context, |cx| self.reader.fetch(cx, generation, url));
            }
            Operation::OpenTextBrowser { url } => {
                if let Some(event) = self.interaction.open_text_browser(url) {
                    context.emit(event);
                }
            }
            Operation::ForceRedrawTerminal => context.force_redraw(),
            Operation::PersistCredential { credential } => {
                self.persist_credential(&credential, context);
            }
            Operation::SetCredential { credential } => self.set_credential(&credential, context),
        }
    }
}

impl Drivers {
    fn register_request<F>(context: &mut DriverContext<Event>, make_request: F)
    where
        F: FnOnce(RequestContext) -> (RequestKind, RequestFuture),
    {
        context.request(|cx| {
            let request_id = cx.id();
            let (kind, request) = make_request(cx);
            let started = Event::RequestEmitted {
                request_id,
                kind: kind.clone(),
            };
            (started, async move {
                Event::RequestCompleted {
                    request_id,
                    kind,
                    result: request.await,
                }
            })
        });
    }

    fn persist_credential(
        &self,
        credential: &Verified<Credential>,
        context: &DriverContext<Event>,
    ) {
        if let Err(error) = self.cache.persist_credential(credential) {
            context.emit(Event::OperationFailed {
                error: OperationError::PersistCredential(error),
            });
        }
    }

    fn set_credential(
        &mut self,
        credential: &Verified<Credential>,
        context: &DriverContext<Event>,
    ) {
        match self.auth.set_credential(self.clock.now(), credential) {
            Ok(()) => {
                self.feed.restart_events_if_started();
                context.emit(Event::ApiCredentialConfigured);
            }
            Err(error) => context.emit(Event::OperationFailed {
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
}
