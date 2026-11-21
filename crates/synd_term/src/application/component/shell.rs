use super::{Component, RenderContext};
use crate::{
    command::{Command, FeedsCommand},
    event::EventKind,
    keymap::Layer,
};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    widgets::{Block, Widget},
};
use synd_client::SyndApiError;
use tracing::{error, warn};
use url::Url;

use crate::{
    application::effects::Effects,
    application::{Direction, Features, RequestError, RequestKind},
    auth::AuthenticationProvider,
    command::{FilterTarget, ShellCommand},
    event::{AuthEvent, Event, OperationError},
    operation::Operation,
    ui::{
        theme::{Palette, Theme},
        widgets::{
            filter::FilterWidget,
            status::StatusLineWidget,
            tabs::{Tab, TabsWidget},
        },
    },
};

/// API authentication state that gates normal application behavior.
#[derive(Debug)]
pub(crate) enum AuthenticationState {
    NotRequired,
    Required,
    RequestingDeviceFlow {
        provider: AuthenticationProvider,
    },
    DeviceFlow {
        provider: AuthenticationProvider,
        verification_url: Url,
        user_code: String,
    },
    Authenticated,
}

/// API-access change that determines the remote work following credential setup.
pub(crate) enum ApiAccessTransition {
    Established,
    Reconfigured,
}

impl AuthenticationState {
    fn begin(&mut self, provider: AuthenticationProvider) -> bool {
        if !matches!(self, Self::Required) {
            return false;
        }
        *self = Self::RequestingDeviceFlow { provider };
        true
    }

    fn apply_event(&mut self, event: AuthEvent) -> [Operation; 2] {
        match event {
            AuthEvent::DeviceFlowAuthorizationReceived {
                provider,
                verification_url,
                device_authorization,
            } => {
                let Self::RequestingDeviceFlow {
                    provider: expected_provider,
                } = self
                else {
                    panic!("device authorization received outside an authentication request");
                };
                assert_eq!(
                    *expected_provider, provider,
                    "device authorization provider did not match its request"
                );
                *self = Self::DeviceFlow {
                    provider,
                    verification_url: verification_url.clone(),
                    user_code: device_authorization.user_code.clone(),
                };
                [
                    Operation::OpenBrowser {
                        url: verification_url,
                    },
                    Operation::PollDeviceFlowAccessToken {
                        provider,
                        device_authorization,
                    },
                ]
            }
            AuthEvent::DeviceFlowCredentialReceived { credential } => {
                assert!(
                    matches!(self, Self::DeviceFlow { .. }),
                    "credential received outside a device flow"
                );
                [
                    Operation::PersistCredential {
                        credential: credential.clone(),
                    },
                    Operation::SetCredential { credential },
                ]
            }
        }
    }

    fn request_failed(&mut self, kind: &RequestKind, error: &RequestError) {
        let returns_to_required = match (&*self, kind, error) {
            (
                Self::RequestingDeviceFlow { provider: active },
                RequestKind::StartDeviceFlow { provider },
                _,
            ) => {
                assert_eq!(active, provider, "failed device flow provider changed");
                true
            }
            (
                Self::DeviceFlow {
                    provider: active, ..
                },
                RequestKind::PollDeviceFlowAccessToken { provider },
                _,
            ) => {
                assert_eq!(active, provider, "failed token poll provider changed");
                true
            }
            (
                Self::Authenticated,
                _,
                RequestError::SyndApi(synd_client::SyndApiError::Unauthorized { .. }),
            ) => true,
            _ => false,
        };
        if returns_to_required {
            *self = Self::Required;
        }
    }

    fn api_credential_configured(&mut self) -> ApiAccessTransition {
        match self {
            Self::Required | Self::DeviceFlow { .. } => {
                *self = Self::Authenticated;
                ApiAccessTransition::Established
            }
            Self::Authenticated => ApiAccessTransition::Reconfigured,
            Self::NotRequired => {
                panic!("credential configured for a transport-trusted client")
            }
            Self::RequestingDeviceFlow { .. } => {
                panic!("credential configured before device-flow completion")
            }
        }
    }

    fn operation_failed(&mut self, error: &OperationError) {
        if matches!(
            (&*self, error),
            (Self::DeviceFlow { .. }, OperationError::SetCredential(_))
        ) {
            *self = Self::Required;
        }
    }
}

/// Global terminal interaction and status state shared across domain components.
pub(crate) struct ShellComponent {
    pub(in crate::application) theme: Theme,
    pub(in crate::application) should_quit: bool,
    pub(super) authentication: AuthenticationState,
    authentication_providers: Vec<AuthenticationProvider>,
    selected_authentication_provider: usize,
    pub(crate) tabs: TabsWidget,
    pub(crate) filter: FilterWidget,
    pub(crate) prompt: StatusLineWidget,
}

impl ShellComponent {
    pub(super) fn new(
        features: &Features,
        filter: FilterWidget,
        theme: Theme,
        authentication: AuthenticationState,
    ) -> Self {
        Self {
            theme,
            should_quit: false,
            authentication,
            authentication_providers: vec![
                AuthenticationProvider::Gh,
                AuthenticationProvider::Google,
            ],
            selected_authentication_provider: 0,
            tabs: TabsWidget::new(features),
            filter,
            prompt: StatusLineWidget::new(),
        }
    }

    pub(in crate::application) fn visible_tab(&self) -> Option<Tab> {
        match self.authentication() {
            AuthenticationState::NotRequired | AuthenticationState::Authenticated => {
                Some(self.tabs.current())
            }
            _ => None,
        }
    }

    pub(crate) fn authentication(&self) -> &AuthenticationState {
        &self.authentication
    }

    pub(crate) fn authentication_providers(&self) -> &[AuthenticationProvider] {
        &self.authentication_providers
    }

    pub(crate) fn selected_authentication_provider_index(&self) -> usize {
        self.selected_authentication_provider
    }

    pub(in crate::application) fn selected_authentication_provider(
        &self,
    ) -> AuthenticationProvider {
        self.authentication_providers[self.selected_authentication_provider]
    }

    pub(in crate::application) fn start_authentication(&mut self) -> Option<Operation> {
        let provider = self.selected_authentication_provider();
        self.authentication
            .begin(provider)
            .then_some(Operation::StartDeviceFlow { provider })
    }

    pub(in crate::application) fn apply_auth_event(&mut self, event: AuthEvent) -> [Operation; 2] {
        self.authentication.apply_event(event)
    }

    pub(in crate::application) fn apply_request_failure(
        &mut self,
        kind: &RequestKind,
        error: &RequestError,
    ) {
        self.authentication.request_failed(kind, error);
    }

    pub(in crate::application) fn api_credential_configured(&mut self) -> ApiAccessTransition {
        self.authentication.api_credential_configured()
    }

    pub(in crate::application) fn apply_operation_failure(&mut self, error: &OperationError) {
        self.authentication.operation_failed(error);
    }

    pub(in crate::application) fn move_authentication_provider(&mut self, direction: Direction) {
        if !matches!(self.authentication, AuthenticationState::Required) {
            return;
        }
        self.selected_authentication_provider = direction.apply(
            self.selected_authentication_provider,
            self.authentication_providers.len(),
        );
    }

    pub(in crate::application) fn apply_command(&mut self, command: ShellCommand) -> Effects {
        match command {
            ShellCommand::ShowError(message) => self.prompt.set_error_message(message),
            ShellCommand::Quit => self.should_quit = true,
            ShellCommand::Authenticate => return Effects::operations(self.start_authentication()),
            ShellCommand::MoveAuthenticationProvider(direction) => {
                self.move_authentication_provider(direction);
            }
            ShellCommand::MoveTabSelection(direction) => {
                let previous = self.tabs.current();
                let tab = self.tabs.move_selection(direction);
                if tab != previous {
                    return Event::TabChanged(tab).into();
                }
            }
            ShellCommand::RotateTheme => self.rotate_theme(),
            ShellCommand::ForceRedraw => {
                return Effects::operations(Operation::ForceRedrawTerminal);
            }
        }
        Effects::NONE
    }

    pub(crate) fn current_filter_target(&self) -> FilterTarget {
        match self.tabs.current() {
            Tab::Feeds | Tab::Entries => FilterTarget::Feeds,
            Tab::Gh => FilterTarget::GhNotifications,
        }
    }

    pub(in crate::application) fn rotate_theme(&mut self) {
        let palette = match self.theme.name {
            "ferra" => Palette::solarized_dark(),
            "solarized_dark" => Palette::helix(),
            "helix" => Palette::dracula(),
            "dracula" => Palette::eldritch(),
            _ => Palette::ferra(),
        };
        self.theme = Theme::with_palette(palette);
    }
}

impl ShellComponent {
    pub(super) fn request_error_message(error: &RequestError) -> String {
        match error {
            RequestError::SyndApi(error) => Self::synd_api_error_message(error),
            RequestError::Authentication(error) => error.to_string(),
            RequestError::Gh(error) => error.to_string(),
        }
    }

    pub(super) fn operation_error_message(error: &OperationError) -> String {
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

impl Component for ShellComponent {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        Block::new()
            .style(cx.ui.theme.base)
            .render(cx.frame.area, buf);
        self.prompt
            .render(cx.frame.status, buf, &cx.ui, || cx.keymap.hints(&cx.layers));
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::App, Layer::Global]
    }
    fn subscriptions(&self) -> &'static [EventKind] {
        &[
            EventKind::RequestCompleted,
            EventKind::Auth,
            EventKind::ApiCredentialConfigured,
            EventKind::CredentialRefreshed,
            EventKind::CredentialRefreshFailed,
            EventKind::OperationFailed,
        ]
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::RequestCompleted {
                kind,
                result: Err(error),
                ..
            } => {
                self.apply_request_failure(kind, error);
                let message = Self::request_error_message(error);
                error!(?kind, %message, "request failed");
                self.prompt.set_error_message(message);
            }
            Event::Auth { event, .. } => {
                return Effects::operations(self.apply_auth_event(event.clone()));
            }
            Event::ApiCredentialConfigured => {
                return match self.api_credential_configured() {
                    ApiAccessTransition::Established => Event::ApiAccessReady.into(),
                    ApiAccessTransition::Reconfigured => {
                        Command::Feeds(FeedsCommand::RefreshTimeline).into()
                    }
                };
            }
            Event::CredentialRefreshed { credential } => {
                return Effects::operations([
                    Operation::PersistCredential {
                        credential: credential.clone(),
                    },
                    Operation::SetCredential {
                        credential: credential.clone(),
                    },
                ]);
            }
            Event::CredentialRefreshFailed { error } => {
                warn!(%error, "credential refresh failed");
                self.prompt.set_error_message(error.to_string());
            }
            Event::OperationFailed { error } => {
                self.apply_operation_failure(error);
                let message = Self::operation_error_message(error);
                error!(%message, "application operation failed");
                self.prompt.set_error_message(message);
            }
            _ => {}
        }
        Effects::NONE
    }
}
