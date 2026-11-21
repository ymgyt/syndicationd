use futures_util::future::BoxFuture;
use tokio::sync::mpsc;

use crate::{
    application::{RequestError, RequestId, RequestKind},
    event::{AuthEvent, Event, FeedRequestEvent, FeedsEvent, GhEvent},
};

pub(super) type RequestFuture = BoxFuture<'static, Result<(), RequestError>>;
pub(super) type JobFuture = BoxFuture<'static, ()>;

/// Emits domain events correlated to one registered external request.
pub(super) struct RequestContext {
    request_id: RequestId,
    kind: RequestKind,
    event_tx: mpsc::UnboundedSender<Event>,
}

impl RequestContext {
    pub(super) fn new(
        request_id: RequestId,
        kind: RequestKind,
        event_tx: mpsc::UnboundedSender<Event>,
    ) -> Self {
        Self {
            request_id,
            kind,
            event_tx,
        }
    }

    pub(super) fn emit_auth(&self, event: AuthEvent) {
        match (&self.kind, &event) {
            (
                RequestKind::StartDeviceFlow { provider: expected },
                AuthEvent::DeviceFlowAuthorizationReceived { provider, .. },
            ) => assert_eq!(
                expected, provider,
                "device authorization provider did not match its request"
            ),
            (
                RequestKind::PollDeviceFlowAccessToken { .. },
                AuthEvent::DeviceFlowCredentialReceived { .. },
            ) => {}
            _ => panic!("authentication event did not match its request"),
        }
        self.emit(Event::Auth {
            request_id: self.request_id,
            event,
        });
    }

    pub(super) fn emit_feeds(&self, event: FeedRequestEvent) {
        match (&self.kind, &event) {
            (
                RequestKind::SubscribeFeed { url: expected },
                FeedRequestEvent::FeedSubscribed { url },
            ) => assert_eq!(expected, url, "subscribed feed did not match its request"),
            (
                RequestKind::UnsubscribeFeed { url: expected },
                FeedRequestEvent::FeedUnsubscribed { url },
            ) => assert_eq!(expected, url, "unsubscribed feed did not match its request"),
            (RequestKind::FetchSubscription, FeedRequestEvent::SubscriptionFetched { .. })
            | (
                RequestKind::CatchUpTimeline { .. },
                FeedRequestEvent::TimelineChangesFetched { .. },
            )
            | (
                RequestKind::FetchTimelineWindow { .. },
                FeedRequestEvent::TimelineWindowChunkFetched { .. },
            ) => {}
            _ => panic!("feed event did not match its request"),
        }
        self.emit(Event::Feeds(FeedsEvent::Request {
            request_id: self.request_id,
            event,
        }));
    }

    pub(super) fn emit_gh(&self, event: GhEvent) {
        match (&self.kind, &event) {
            (RequestKind::FetchGhNotifications { .. }, GhEvent::NotificationsFetched { .. })
            | (RequestKind::FetchGhIssue { .. }, GhEvent::IssueFetched { .. })
            | (RequestKind::FetchGhPullRequest { .. }, GhEvent::PullRequestFetched { .. }) => {}
            (
                RequestKind::MarkGhNotificationAsDone { id: expected },
                GhEvent::NotificationMarkedAsDone { notification_id },
            ) => assert_eq!(
                expected, notification_id,
                "completed GitHub notification did not match its request"
            ),
            _ => panic!("GitHub event did not match its request"),
        }
        self.emit(Event::Gh {
            generation: self.kind.gh_generation(),
            request_id: self.request_id,
            event,
        });
    }

    fn emit(&self, event: Event) {
        self.event_tx
            .send(event)
            .expect("Drivers owns the event receiver");
    }
}
