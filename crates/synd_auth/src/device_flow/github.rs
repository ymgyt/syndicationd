use std::{borrow::Cow, future::Future, time::Duration};

use http::{StatusCode, Uri};
use reqwest::Client;
use tracing::debug;

use crate::device_flow::{
    DeviceAccessTokenErrorResponse, DeviceAccessTokenRequest, DeviceAccessTokenResponse,
    DeviceAuthorizationRequest, DeviceAuthorizationResponse,
};

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// <https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow>
#[derive(Clone)]
pub struct DeviceFlow {
    client: Client,
    client_id: Cow<'static, str>,
    device_authorization_endpoint: Option<Cow<'static, str>>,
    token_endpoint: Option<Cow<'static, str>>,
}

impl DeviceFlow {
    const DEVICE_AUTHORIZATION_ENDPOINT: &'static str = "https://github.com/login/device/code";
    const TOKEN_ENDPOINT: &'static str = "https://github.com/login/oauth/access_token";

    pub fn new(client_id: impl Into<Cow<'static, str>>) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        Self {
            client,
            client_id: client_id.into(),
            device_authorization_endpoint: None,
            token_endpoint: None,
        }
    }

    #[must_use]
    pub fn with_device_authorization_endpoint(
        self,
        endpoint: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            device_authorization_endpoint: Some(endpoint.into()),
            ..self
        }
    }

    #[must_use]
    pub fn with_token_endpoint(self, endpoint: impl Into<Cow<'static, str>>) -> Self {
        Self {
            token_endpoint: Some(endpoint.into()),
            ..self
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn device_authorize_request(&self) -> anyhow::Result<DeviceAuthorizationResponse> {
        tracing::debug!("Sending request");

        // https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
        let scope = "user:email";

        let response = self
            .client
            .post(
                self.device_authorization_endpoint
                    .as_deref()
                    .unwrap_or(Self::DEVICE_AUTHORIZATION_ENDPOINT),
            )
            .header(http::header::ACCEPT, "application/json")
            .form(&DeviceAuthorizationRequest {
                client_id: self.client_id.clone(),
                scope: scope.into(),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<DeviceAuthorizationResponse>()
            .await?;

        tracing::debug!("Got response");

        Ok(response)
    }

    pub async fn pool_device_access_token(
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
                        "Failed to authenticate. authorization server respond with {err_response:?}"
                    )
                }
            }};
        }

        let response = loop {
            let response = self
                .client
                .post(
                    self.token_endpoint
                        .as_deref()
                        .unwrap_or(Self::TOKEN_ENDPOINT),
                )
                .header(http::header::ACCEPT, "application/json")
                .form(&DeviceAccessTokenRequest::new(
                    &device_code,
                    self.client_id.as_ref(),
                ))
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
                StatusCode::BAD_REQUEST => {
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

    #[tracing::instrument(skip_all)]
    pub async fn device_flow<F, Fut>(self, callback: F) -> anyhow::Result<DeviceAccessTokenResponse>
    where
        F: FnOnce(Uri, String) -> Fut,
        Fut: Future<Output = ()>,
    {
        let DeviceAuthorizationResponse {
            device_code,
            user_code,
            verification_uri,
            interval,
            ..
        } = self.device_authorize_request().await?;

        callback(verification_uri, user_code).await;

        self.pool_device_access_token(device_code, interval).await
    }
}
