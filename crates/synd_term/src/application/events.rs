use itertools::Itertools;
use synd_client::SyndApiError;
use tracing::{error, info_span, instrument, warn};

use crate::event::{ApiEvent, AuthApiEvent, Event};

use super::{Application, RequestSequence};

impl Application {
    #[instrument(skip_all)]
    pub(super) fn apply_event(&mut self, event: Event) {
        let _guard = info_span!("apply_event", %event).entered();

        match event {
            // The event loop renders after every event; resize needs no extra handling
            Event::TerminalResized => {}
            Event::RenderThrobber => {
                self.drivers.reset_throbber();
            }
            Event::Idle => {
                self.handle_idle();
            }
            Event::Api { request_seq, event } => {
                self.apply_api_event(request_seq, event);
            }
            Event::CredentialRefreshed { credential } => {
                self.set_credential(credential);
                if self.drivers.restart_feed_events_if_running() {
                    let operations = self.components.mark_timeline_dirty();
                    self.perform_operations(operations);
                }
            }
            Event::FeedSubscriptionEditorClosed { input } => {
                let operations = self
                    .components
                    .apply_feed_subscription_editor_closed(input.as_str());
                self.perform_operations(operations);
            }
            Event::FeedEditionEditorClosed { input } => {
                let operations = self
                    .components
                    .apply_feed_edition_editor_closed(input.as_str());
                self.perform_operations(operations);
            }
            Event::EntryFetchStarted {
                request_seq,
                populate,
            } => {
                self.components
                    .apply_entry_fetch_started(request_seq, populate);
            }
            Event::RegistryFeed { event } => {
                let operations = self.components.apply_feed_event(event);
                self.perform_operations(operations);
            }
            Event::TimelineSyncDebounced => {
                let operation = self.components.feeds.timeline_sync_debounced();
                self.perform_operation(operation);
            }
            Event::Error { message } => {
                self.handle_error_message(message, None);
            }
            Event::SyndApiError { error, request_seq } => {
                self.components.apply_synd_api_error(request_seq);
                let message = Self::synd_api_error_message(error.as_ref());
                self.handle_error_message(message, Some(request_seq));
            }
            Event::OauthApiError { error, request_seq } => {
                self.handle_error_message(error.to_string(), Some(request_seq));
            }
            Event::GithubApiError { error, request_seq } => {
                self.handle_error_message(error.to_string(), Some(request_seq));
            }
        }
    }

    fn apply_api_event(&mut self, request_seq: RequestSequence, event: ApiEvent) {
        self.drivers.remove_in_flight(request_seq);

        match event {
            ApiEvent::Auth(event) => self.apply_auth_api_event(event),
            ApiEvent::Feeds(event) => {
                let entries_first = self.next_entries_first(0);
                let operations = self.components.apply_feeds_api_event(
                    request_seq,
                    event,
                    self.config.feeds_per_pagination,
                    entries_first,
                    self.config.entries_limit,
                );
                self.perform_operations(operations);
            }
            ApiEvent::GitHub(event) => {
                let operations = self.components.apply_github_api_event(event);
                self.perform_operations(operations);
            }
        }
    }

    fn apply_auth_api_event(&mut self, event: AuthApiEvent) {
        match event {
            AuthApiEvent::DeviceFlowAuthorizationReceived {
                provider,
                device_authorization,
            } => {
                let operations = self
                    .components
                    .apply_device_flow_authorization_received(provider, *device_authorization);
                self.perform_operations(operations);
            }
            AuthApiEvent::DeviceFlowCredentialReceived { credential } => {
                self.complete_device_authorize_flow(credential);
            }
        }
    }

    pub(super) fn handle_error_message(
        &mut self,
        error_message: String,
        request_seq: Option<RequestSequence>,
    ) {
        error!("{error_message}");

        if let Some(request_seq) = request_seq {
            self.drivers.remove_in_flight(request_seq);
        }

        self.components
            .shell
            .prompt
            .set_error_message(error_message);
    }

    fn synd_api_error_message(error: &SyndApiError) -> String {
        match error {
            SyndApiError::Unauthorized { url } => {
                format!(
                    "{} unauthorized. local feed API session is invalid",
                    url.as_ref().map(ToString::to_string).unwrap_or_default(),
                )
            }
            SyndApiError::BuildRequest(err) => {
                format!("build request failed: {err} this is a BUG")
            }
            SyndApiError::HttpStatus { status, url } => {
                format!(
                    "HTTP status client error ({status}) for url ({})",
                    url.as_ref().map(ToString::to_string).unwrap_or_default()
                )
            }
            SyndApiError::Graphql { errors } => errors.iter().map(ToString::to_string).join(", "),
            SyndApiError::SubscribeFeed(err) => err.to_string(),
            SyndApiError::OpenSession(err) => format!("session open rejected: {err:?}"),
            SyndApiError::RenewSession(err) => format!("session renew rejected: {err:?}"),
            SyndApiError::CloseSession(err) => format!("session close rejected: {err:?}"),
            SyndApiError::MissingCredential
            | SyndApiError::InvalidHeader(_)
            | SyndApiError::InvalidUrl(_)
            | SyndApiError::WebSocket(_)
            | SyndApiError::Json(_)
            | SyndApiError::UnexpectedResponse { .. }
            | SyndApiError::TlsWebSocketUnsupported
            | SyndApiError::UnsupportedWebSocketScheme { .. }
            | SyndApiError::SetWebSocketScheme
            | SyndApiError::SubscriptionProtocol { .. } => error.to_string(),
        }
    }
}
