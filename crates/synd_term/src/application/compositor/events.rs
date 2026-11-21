use itertools::Itertools;
use synd_client::SyndApiError;
use tracing::{error, info_span, instrument, warn};

use crate::{
    command::FilterTarget,
    event::{Event, FeedRequestEvent, FeedsEvent, OperationError},
    operation::{Operation, Operations},
};

use super::Compositor;
use crate::application::{RequestError, RequestId, RequestKind, component::ApiAccessTransition};

impl Compositor {
    #[instrument(skip_all)]
    pub(in crate::application) fn apply_event(&mut self, event: Event) -> Operations {
        let _guard = info_span!("apply_event", %event).entered();

        let operations = match event {
            Event::TabChanged(tab) => self
                .components
                .feeds
                .tab_changed(tab, self.feeds_per_pagination)
                .into(),
            Event::FilterChanged(FilterTarget::Feeds) => {
                let filter = self.components.shell.filter.feed_filterer();
                self.components.feeds.apply_filter(filter);
                Operations::Nop
            }
            Event::FilterChanged(FilterTarget::GhNotifications) => {
                let filter = self.components.shell.filter.gh_notification_filterer();
                self.components.gh.apply_filter(filter).into()
            }
            Event::RequestEmitted { request_id, kind } => {
                self.components.shell.in_flight.register(request_id, kind);
                Operations::Nop
            }
            Event::RequestCompleted { request_id, result } => {
                self.apply_request_completed(request_id, result).into()
            }
            Event::Auth { request_id, event } => {
                self.components
                    .shell
                    .in_flight
                    .correlate_auth_event(request_id, &event);
                self.components.shell.apply_auth_event(event).into()
            }
            Event::Feeds(FeedsEvent::Request { request_id, event }) => {
                self.apply_feed_request_event(request_id, event).into()
            }
            Event::Feeds(FeedsEvent::Push { event }) => {
                self.components.apply_feed_push(event).into()
            }
            Event::Gh { request_id, event } => {
                let kind = self
                    .components
                    .shell
                    .in_flight
                    .correlate_gh_event(request_id, &event);
                if self.components.gh.is_stale(kind) {
                    Operations::Nop
                } else {
                    self.components.apply_gh_event(event)
                }
            }
            Event::FeedSubscriptionEditorClosed { input } => self
                .components
                .apply_feed_subscription_editor_closed(input.as_str())
                .into(),
            Event::FeedEditionEditorClosed { input } => self
                .components
                .apply_feed_edition_editor_closed(input.as_str())
                .into(),
            Event::ApiCredentialConfigured => self.apply_api_credential_configured(),
            Event::CredentialRefreshed { credential } => [
                Operation::PersistCredential {
                    credential: credential.clone(),
                },
                Operation::SetCredential { credential },
            ]
            .into(),
            Event::CredentialRefreshFailed { error } => {
                warn!(%error, "credential refresh failed");
                self.show_error_message(error.to_string());
                Operations::Nop
            }
            Event::OperationFailed { error } => {
                self.apply_operation_failure(&error);
                Operations::Nop
            }
        };
        self.sync_keymap();
        operations
    }

    fn apply_feed_request_event(
        &mut self,
        request_id: RequestId,
        event: FeedRequestEvent,
    ) -> Option<Operation> {
        self.components
            .shell
            .in_flight
            .correlate_feed_event(request_id, &event);
        self.components.apply_feed_request_event(
            event,
            self.feeds_per_pagination,
            self.entries_limit,
        )
    }

    fn apply_request_completed(
        &mut self,
        request_id: RequestId,
        result: Result<(), RequestError>,
    ) -> Option<Operation> {
        let kind = self
            .components
            .shell
            .in_flight
            .complete(request_id)
            .into_kind();
        if self.components.gh.is_stale(&kind) {
            return None;
        }
        let succeeded = match result {
            Ok(()) => true,
            Err(error) => {
                if matches!(kind, RequestKind::FetchGhNotifications { .. }) {
                    self.components.gh.page_failed();
                }
                self.apply_request_failure(&kind, &error);
                false
            }
        };

        match kind {
            RequestKind::FetchTimelineWindow { .. } => {
                self.components.feeds.complete_timeline_window(succeeded)
            }
            RequestKind::CatchUpTimeline { .. } => {
                self.components.feeds.complete_timeline_catch_up(succeeded)
            }
            _ => None,
        }
    }

    fn apply_request_failure(&mut self, kind: &RequestKind, error: &RequestError) {
        let message = Self::request_error_message(error);
        self.components.shell.apply_request_failure(kind, error);
        error!(?kind, error = %message, "request failed");
        self.show_error_message(message);
    }

    fn apply_api_credential_configured(&mut self) -> Operations {
        match self.components.shell.api_credential_configured() {
            ApiAccessTransition::Established => self.bootstrap(),
            ApiAccessTransition::Reconfigured => self.components.feeds.refresh_timeline().into(),
        }
    }

    fn apply_operation_failure(&mut self, error: &OperationError) {
        self.components.shell.apply_operation_failure(error);
        let message = Self::operation_error_message(error);
        error!(%message, "application operation failed");
        self.show_error_message(message);
    }

    fn show_error_message(&mut self, message: String) {
        self.components.shell.prompt.set_error_message(message);
    }

    fn request_error_message(error: &RequestError) -> String {
        match error {
            RequestError::SyndApi(error) => Self::synd_api_error_message(error),
            RequestError::Authentication(error) => error.to_string(),
            RequestError::Gh(error) => error.to_string(),
        }
    }

    fn operation_error_message(error: &OperationError) -> String {
        match error {
            OperationError::OpenFeedSubscriptionEditor(error)
            | OperationError::OpenFeedEditionEditor(error) => error.to_string(),
            OperationError::OpenBrowser(error) => format!("open browser: {error}"),
            OperationError::OpenTextBrowser(error) => format!("open text browser: {error}"),
            OperationError::PersistCredential(error) => error.to_string(),
            OperationError::SetCredential(error) => Self::synd_api_error_message(error),
        }
    }

    fn synd_api_error_message(error: &SyndApiError) -> String {
        match error {
            SyndApiError::Unauthorized { url } => format!(
                "{} unauthorized. local feed API session is invalid",
                url.as_ref().map_or_default(ToString::to_string),
            ),
            SyndApiError::BuildRequest(error) => {
                format!("build request failed: {error} this is a BUG")
            }
            SyndApiError::SendRequest(error) => format!("request failed: {error}"),
            SyndApiError::DecodeResponse(error) => {
                format!("decode response failed: {error}")
            }
            SyndApiError::HttpStatus { status, url } => format!(
                "HTTP status client error ({status}) for url ({})",
                url.as_ref().map_or_default(ToString::to_string)
            ),
            SyndApiError::Graphql { errors } => errors.iter().map(ToString::to_string).join(", "),
            SyndApiError::OpenSession(error) => format!("session open rejected: {error:?}"),
            SyndApiError::RenewSession(error) => format!("session renew rejected: {error:?}"),
            SyndApiError::CloseSession(error) => format!("session close rejected: {error:?}"),
            _ => error.to_string(),
        }
    }
}
