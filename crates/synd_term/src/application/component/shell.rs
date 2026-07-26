use url::Url;

use crate::{
    application::{
        Direction, Features, InFlightRequests, RequestError, RequestKind, TerminalFocus,
        state::State,
    },
    auth::AuthenticationProvider,
    command::FilterTarget,
    config::Categories,
    event::{AuthEvent, OperationError},
    operation::Operation,
    ui::{
        theme::{Palette, Theme},
        widgets::{
            filter::{FilterWidget, Filterer},
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
    pub(in crate::application) categories: Categories,
    state: State,
    authentication: AuthenticationState,
    authentication_providers: Vec<AuthenticationProvider>,
    selected_authentication_provider: usize,
    pub(crate) in_flight: InFlightRequests,
    pub(crate) tabs: TabsWidget,
    pub(crate) filter: FilterWidget,
    pub(crate) prompt: StatusLineWidget,
}

impl ShellComponent {
    pub(super) fn new(
        features: &Features,
        theme: Theme,
        categories: Categories,
        authentication: AuthenticationState,
    ) -> Self {
        Self {
            theme,
            categories,
            state: State::new(),
            authentication,
            authentication_providers: vec![
                AuthenticationProvider::Gh,
                AuthenticationProvider::Google,
            ],
            selected_authentication_provider: 0,
            in_flight: InFlightRequests::new(),
            tabs: TabsWidget::new(features),
            filter: FilterWidget::new(),
            prompt: StatusLineWidget::new(),
        }
    }

    pub(crate) fn authentication(&self) -> &AuthenticationState {
        &self.authentication
    }

    pub(in crate::application) fn permits_main_ui(&self) -> bool {
        matches!(
            self.authentication,
            AuthenticationState::NotRequired | AuthenticationState::Authenticated
        )
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

    pub(in crate::application) fn quit(&mut self) {
        self.state.should_quit = true;
    }

    pub(in crate::application) fn take_should_quit(&mut self) -> bool {
        std::mem::take(&mut self.state.should_quit)
    }

    pub(in crate::application) fn focus(&self) -> TerminalFocus {
        self.state.focus()
    }

    pub(in crate::application) fn focus_gained(&mut self) {
        self.state.focus_gained();
    }

    pub(in crate::application) fn focus_lost(&mut self) {
        self.state.focus_lost();
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

    pub(in crate::application) fn move_tab_selection(&mut self, direction: Direction) -> Tab {
        self.tabs.move_selection(direction)
    }

    pub(in crate::application) fn move_filter_requirement(
        &mut self,
        direction: Direction,
    ) -> Filterer {
        self.filter.move_requirement(direction)
    }

    pub(in crate::application) fn active_filterer(&self) -> Filterer {
        self.filter.filterer(self.current_filter_target())
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
