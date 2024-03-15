use std::borrow::Cow;

use http::Uri;
use serde::{Deserialize, Serialize};

pub mod github;
pub mod google;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// <https://datatracker.ietf.org/doc/html/rfc8628#section-3.1>
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceAuthorizationRequest<'s> {
    pub client_id: Cow<'s, str>,
    pub scope: Cow<'s, str>,
}

/// <https://datatracker.ietf.org/doc/html/rfc8628#section-3.2>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAuthorizationResponse {
    /// device verification code
    pub device_code: String,
    /// end user verification code
    pub user_code: String,
    /// end user verification uri on the authorization server
    #[serde(with = "http_serde_ext::uri")]
    pub verification_uri: Uri,
    /// a verification uri that includes user_code which is designed for non-textual transmission.
    // error if there is no field on deserializing, maybe bug on http_serde_ext crate ?
    #[allow(unused)]
    #[serde(with = "http_serde_ext::uri::option", skip_deserializing)]
    pub verification_uri_complete: Option<Uri>,
    /// the lifetime in seconds of the device_code and user_code
    #[allow(unused)]
    pub expires_in: i64,
    /// the minimum amount of time in seconds that the client should wait between polling requests to the token endpoint
    /// if no value is provided, clients must use 5 as the default
    pub interval: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceAccessTokenRequest<'s> {
    /// Value MUST be set to "urn:ietf:params:oauth:grant-type:device_code"
    grant_type: Cow<'static, str>,
    /// The device verification code, "device_code" from the device authorization response
    pub device_code: Cow<'s, str>,
    pub client_id: Cow<'s, str>,

    // vendor extensions
    /// Google require client secret
    /// <https://developers.google.com/identity/gsi/web/guides/devices#obtain_an_id_token_and_refresh_token>
    pub client_secret: Option<Cow<'s, str>>,
}

impl<'s> DeviceAccessTokenRequest<'s> {
    const GRANT_TYPE: &'static str = "urn:ietf:params:oauth:grant-type:device_code";

    #[must_use]
    pub fn new(device_code: &'s str, client_id: &'s str) -> Self {
        Self {
            grant_type: Self::GRANT_TYPE.into(),
            device_code: device_code.into(),
            client_id: client_id.into(),
            client_secret: None,
        }
    }

    /// Configure `grant_type`
    #[must_use]
    pub fn with_grant_type(self, grant_type: impl Into<Cow<'static, str>>) -> Self {
        Self {
            grant_type: grant_type.into(),
            ..self
        }
    }

    /// Configure `client_secret`
    #[must_use]
    pub fn with_client_secret(self, client_secret: impl Into<Cow<'s, str>>) -> Self {
        Self {
            client_secret: Some(client_secret.into()),
            ..self
        }
    }
}

/// Successful Response
/// <https://datatracker.ietf.org/doc/html/rfc6749#section-5.1>
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceAccessTokenResponse {
    /// the access token issued by the authorization server
    pub access_token: String,
    pub token_type: String,
    /// the lifetime in seconds of the access token
    pub expires_in: Option<i64>,

    // OIDC usecase
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

/// <https://datatracker.ietf.org/doc/html/rfc6749#section-5.2>
#[derive(Deserialize, Debug)]
pub struct DeviceAccessTokenErrorResponse {
    pub error: DeviceAccessTokenErrorCode,
    #[allow(unused)]
    pub error_description: Option<String>,
    // error if there is no field on deserializing, maybe bug on http_serde_ext crate ?
    #[allow(unused)]
    #[serde(with = "http_serde_ext::uri::option", skip_deserializing)]
    pub error_uri: Option<Uri>,
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceAccessTokenErrorCode {
    AuthorizationPending,
    SlowDown,
    AccessDenied,
    ExpiredToken,
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,
    IncorrectDeviceCode,
}

impl DeviceAccessTokenErrorCode {
    ///  The `authorization_pending` and `slow_down` error codes define particularly unique behavior, as they indicate that the OAuth client should continue to poll the token endpoint by repeating the token request (implementing the precise behavior defined above)
    pub fn should_continue_to_poll(&self) -> bool {
        use DeviceAccessTokenErrorCode::{AuthorizationPending, SlowDown};
        *self == AuthorizationPending || *self == SlowDown
    }
}
