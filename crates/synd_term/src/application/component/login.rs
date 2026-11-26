use super::RenderContext;
use super::Ui;
use crate::application::ComponentId;
use crate::{
    application::{RequestError, RequestKind, effects::Effects},
    auth::{AuthenticationProvider, Credential, Verified},
    command::{Command, LoginCommand},
    event::{AuthEvent, Event, EventKind, OperationError},
    keymap::Layer,
    operation::Operation,
    ui::widgets::authentication,
};
use tuiframework::Component;
use url::Url;

/// Owns the login screen and the device flow that grants access to the API.
pub(in crate::application) struct LoginComponent {
    state: AuthenticationState,
    selected_provider: usize,
}

/// API authentication state that gates normal application behavior.
#[derive(Debug)]
enum AuthenticationState {
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

impl AuthenticationState {
    fn begin(&mut self, provider: AuthenticationProvider) -> bool {
        if !matches!(self, Self::Required) {
            return false;
        }
        *self = Self::RequestingDeviceFlow { provider };
        true
    }

    fn apply_event(&mut self, event: &AuthEvent) -> [Operation; 2] {
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
                    expected_provider, provider,
                    "device authorization provider did not match its request"
                );
                *self = Self::DeviceFlow {
                    provider: *provider,
                    verification_url: verification_url.clone(),
                    user_code: device_authorization.user_code.clone(),
                };
                [
                    Operation::OpenBrowser {
                        url: verification_url.clone(),
                    },
                    Operation::PollDeviceFlowAccessToken {
                        provider: *provider,
                        device_authorization: device_authorization.clone(),
                    },
                ]
            }
            AuthEvent::DeviceFlowCredentialReceived { credential } => {
                assert!(
                    matches!(self, Self::DeviceFlow { .. }),
                    "credential received outside a device flow"
                );
                configure_credential(credential)
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

    fn api_credential_configured(&mut self) -> Event {
        match self {
            Self::Required | Self::DeviceFlow { .. } => {
                *self = Self::Authenticated;
                Event::ApiAccessReady
            }
            Self::Authenticated => Event::ApiAccessRefreshed,
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

impl LoginComponent {
    const PROVIDERS: [AuthenticationProvider; 2] =
        [AuthenticationProvider::Gh, AuthenticationProvider::Google];

    pub(super) fn new(authentication_required: bool) -> Self {
        Self {
            state: if authentication_required {
                AuthenticationState::Required
            } else {
                AuthenticationState::NotRequired
            },
            selected_provider: 0,
        }
    }

    pub(in crate::application) fn has_api_access(&self) -> bool {
        matches!(
            self.state,
            AuthenticationState::NotRequired | AuthenticationState::Authenticated
        )
    }

    pub(super) fn apply_command(&mut self, command: LoginCommand) -> Effects {
        match command {
            LoginCommand::Authenticate => {
                let provider = Self::PROVIDERS[self.selected_provider];
                if self.state.begin(provider) {
                    return Effects::NONE.operation(Operation::StartDeviceFlow { provider });
                }
            }
            LoginCommand::MoveProvider(direction)
                if matches!(self.state, AuthenticationState::Required) =>
            {
                self.selected_provider =
                    direction.apply(self.selected_provider, Self::PROVIDERS.len());
            }
            LoginCommand::MoveProvider(_) => {}
        }
        Effects::NONE
    }
}

impl Component<Ui> for LoginComponent {
    fn apply_command(&mut self, command: Command) -> Effects {
        match command {
            Command::Login(action) => self.apply_command(action),
            _ => Effects::NONE,
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        match &self.state {
            AuthenticationState::Required | AuthenticationState::RequestingDeviceFlow { .. } => {
                authentication::render_login(
                    cx.frame.content,
                    buf,
                    cx.ui.theme,
                    &Self::PROVIDERS,
                    self.selected_provider,
                );
            }
            AuthenticationState::DeviceFlow {
                verification_url,
                user_code,
                ..
            } => authentication::render_device_flow(
                cx.frame.content,
                buf,
                cx.ui.theme,
                verification_url.as_str(),
                user_code,
            ),
            AuthenticationState::NotRequired | AuthenticationState::Authenticated => {}
        }
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::Login]
    }

    fn subscribes(&self, event: &Event) -> bool {
        [
            EventKind::Auth,
            EventKind::ApiCredentialConfigured,
            EventKind::CredentialRefreshed,
            EventKind::RequestCompleted,
            EventKind::OperationFailed,
        ]
        .contains(&EventKind::from(event))
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::Auth { event, .. } => Effects::from_operations(self.state.apply_event(event)),
            Event::ApiCredentialConfigured => self.state.api_credential_configured().into(),
            Event::CredentialRefreshed { credential } => {
                Effects::from_operations(configure_credential(credential))
            }
            Event::RequestCompleted {
                kind,
                result: Err(error),
                ..
            } => {
                self.state.request_failed(kind, error);
                if let RequestError::Authentication(error) = error {
                    return Effects::NONE.send(
                        ComponentId::Shell,
                        Command::show_request_error(kind, error.to_string()),
                    );
                }
                Effects::NONE
            }
            Event::OperationFailed { error } => {
                self.state.operation_failed(error);
                Effects::NONE
            }
            _ => Effects::NONE,
        }
    }
}

fn configure_credential(credential: &Verified<Credential>) -> [Operation; 2] {
    [
        Operation::PersistCredential {
            credential: credential.clone(),
        },
        Operation::SetCredential {
            credential: credential.clone(),
        },
    ]
}
