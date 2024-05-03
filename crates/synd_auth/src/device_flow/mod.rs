use std::{borrow::Cow, time::Duration};

use http::{StatusCode, Uri};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::USER_AGENT;

pub mod provider;

pub trait Provider {
    type DeviceAccessTokenRequest<'d>: Serialize + Send
    where
        Self: 'd;

    fn device_authorization_endpoint(&self) -> Url;
    fn token_endpoint(&self) -> Url;
    fn device_authorization_request(&self) -> DeviceAuthorizationRequest;
    fn device_access_token_request<'d, 'p: 'd>(
        &'p self,
        device_code: &'d str,
    ) -> Self::DeviceAccessTokenRequest<'d>;
}

#[derive(Clone)]
pub struct DeviceFlow<P> {
    provider: P,
    client: Client,
}

impl<P> DeviceFlow<P> {
    pub fn new(provider: P) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        Self { provider, client }
    }
}

impl<P: Provider> DeviceFlow<P> {
    #[tracing::instrument(skip(self))]
    pub async fn device_authorize_request(&self) -> anyhow::Result<DeviceAuthorizationResponse> {
        let response = self
            .client
            .post(self.provider.device_authorization_endpoint())
            .header(http::header::ACCEPT, "application/json")
            .form(&self.provider.device_authorization_request())
            .send()
            .await?
            .error_for_status()?
            .json::<DeviceAuthorizationResponse>()
            .await?;

        Ok(response)
    }

    pub async fn poll_device_access_token(
        &self,
        device_code: String,
        interval: Option<i64>,
    ) -> anyhow::Result<DeviceAccessTokenResponse> {
        // poll to check if user authorized the device
        macro_rules! continue_or_abort {
            ( $response_bytes:ident ) => {{
                let err_response = serde_json::from_slice::<DeviceAccessTokenErrorResponse>(&$response_bytes)?;
                if err_response.error.should_continue_to_poll() {
                    debug!(error_code=?err_response.error,interval, "Continue to poll");

                    let interval = interval.unwrap_or(5);

                    tokio::time::sleep(Duration::from_secs(interval as u64)).await;
                } else {
                    anyhow::bail!(
                        "authorization server or oidc provider respond with {err_response:?}"
                    )
                }
            }};
        }

        let response = loop {
            let response = self
                .client
                .post(self.provider.token_endpoint())
                .header(http::header::ACCEPT, "application/json")
                .form(&self.provider.device_access_token_request(&device_code))
                .send()
                .await?;

            match response.status() {
                StatusCode::OK => {
                    let full = response.bytes().await?;
                    if let Ok(response) = serde_json::from_slice::<DeviceAccessTokenResponse>(&full)
                    {
                        break response;
                    }
                    continue_or_abort!(full);
                }
                // Google return 428(Precondition required)
                StatusCode::BAD_REQUEST | StatusCode::PRECONDITION_REQUIRED => {
                    let full = response.bytes().await?;
                    continue_or_abort!(full);
                }
                other => {
                    let error_msg = response.text().await.unwrap_or_default();
                    anyhow::bail!("Failed to authenticate. authorization server respond with {other} {error_msg}")
                }
            }
        };

        Ok(response)
    }
}

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
    #[serde(with = "http_serde_ext::uri::option", default)]
    pub verification_uri: Option<Uri>,
    /// Google use verification_"url"
    #[serde(with = "http_serde_ext::uri::option", default)]
    pub verification_url: Option<Uri>,
    /// a verification uri that includes `user_code` which is designed for non-textual transmission.
    #[allow(unused)]
    #[serde(with = "http_serde_ext::uri::option", default)]
    pub verification_uri_complete: Option<Uri>,
    /// the lifetime in seconds of the `device_code` and `user_code`
    #[allow(unused)]
    pub expires_in: i64,
    /// the minimum amount of time in seconds that the client should wait between polling requests to the token endpoint
    /// if no value is provided, clients must use 5 as the default
    pub interval: Option<i64>,
}

impl DeviceAuthorizationResponse {
    pub fn verification_uri(&self) -> &Uri {
        self.verification_uri
            .as_ref()
            .or(self.verification_url.as_ref())
            .expect("verification uri or url not found")
    }
}

#[derive(Serialize, Deserialize)]
pub struct DeviceAccessTokenRequest<'s> {
    /// Value MUST be set to "urn:ietf:params:oauth:grant-type:device_code"
    grant_type: Cow<'static, str>,
    /// The device verification code, `device_code` from the device authorization response
    pub device_code: Cow<'s, str>,
    pub client_id: Cow<'s, str>,
}

impl<'s> DeviceAccessTokenRequest<'s> {
    const GRANT_TYPE: &'static str = "urn:ietf:params:oauth:grant-type:device_code";

    #[must_use]
    pub fn new(device_code: impl Into<Cow<'s, str>>, client_id: &'s str) -> Self {
        Self {
            grant_type: Self::GRANT_TYPE.into(),
            device_code: device_code.into(),
            client_id: client_id.into(),
        }
    }
}

/// Successful Response
/// <https://datatracker.ietf.org/doc/html/rfc6749#section-5.1>
#[derive(Serialize, Deserialize, Debug, Clone)]
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
