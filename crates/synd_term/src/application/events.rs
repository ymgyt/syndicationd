use itertools::Itertools;
use synd_client::SyndApiError;
use tracing::{error, info_span, instrument, warn};

use crate::{
    event::{Event, FeedRequestEvent, FeedsEvent, OperationError},
    operation::{Operation, Operations},
};

use super::{Application, RequestError, RequestId, RequestKind, component::ApiAccessTransition};

impl Application {
    #[instrument(skip_all)]
    pub(super) fn apply_event(&mut self, event: Event) -> Operations {
        let _guard = info_span!("apply_event", %event).entered();

        match event {
            Event::TerminalResized => Operations::Nop,
            Event::TerminalFocusGained => {
                self.components.shell.focus_gained();
                Operations::Nop
            }
            Event::TerminalFocusLost => {
                self.components.shell.focus_lost();
                Operations::Nop
            }
            Event::ThrobberTick => {
                self.components.shell.in_flight.tick();
                Operations::Nop
            }
            Event::Idle => {
                self.apply_idle();
                Operations::Nop
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
                self.components
                    .shell
                    .in_flight
                    .correlate_gh_event(request_id, &event);
                self.components.apply_gh_event(event)
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
        }
    }

    fn apply_idle(&mut self) {
        #[cfg(feature = "integration")]
        self.components.shell.quit();
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
            self.config.feeds_per_pagination,
            self.config.entries_limit,
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
        let succeeded = match result {
            Ok(()) => true,
            Err(error) => {
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
            ApiAccessTransition::Established => self.bootstrap().into(),
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
                url.as_ref().map(ToString::to_string).unwrap_or_default(),
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
                url.as_ref().map(ToString::to_string).unwrap_or_default()
            ),
            SyndApiError::Graphql { errors } => errors.iter().map(ToString::to_string).join(", "),
            SyndApiError::OpenSession(error) => format!("session open rejected: {error:?}"),
            SyndApiError::RenewSession(error) => format!("session renew rejected: {error:?}"),
            SyndApiError::CloseSession(error) => format!("session close rejected: {error:?}"),
            _ => error.to_string(),
        }
    }
}
