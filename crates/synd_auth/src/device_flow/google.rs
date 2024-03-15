use std::{borrow::Cow, time::Duration};

use http::StatusCode;
use reqwest::Client;
use tracing::debug;

use crate::device_flow::{
    DeviceAccessTokenErrorResponse, DeviceAccessTokenRequest, DeviceAccessTokenResponse,
    DeviceAuthorizationRequest, DeviceAuthorizationResponse, USER_AGENT,
};

pub struct DeviceFlow {
    client: Client,
    client_id: Cow<'static, str>,
    client_secret: Cow<'static, str>,
}

impl DeviceFlow {
    const DEVICE_AUTHORIZATION_ENDPOINT: &'static str = "https://oauth2.googleapis.com/device/code";
    const TOKEN_ENDPOINT: &'static str = "https://oauth2.googleapis.com/token";
    /// <https://developers.google.com/identity/gsi/web/guides/devices#obtain_an_id_token_and_refresh_token>
    const GRANT_TYPE: &'static str = "http://oauth.net/grant_type/device/1.0";

    pub fn new(
        client_id: impl Into<Cow<'static, str>>,
        client_secret: impl Into<Cow<'static, str>>,
    ) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        Self {
            client,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn device_authorize_request(&self) -> anyhow::Result<DeviceAuthorizationResponse> {
        // https://developers.google.com/identity/gsi/web/guides/devices#obtain_a_user_code_and_verification_url
        let scope = "email";
        let response = self
            .client
            .post(Self::DEVICE_AUTHORIZATION_ENDPOINT)
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
                        "Failed to authenticate. authorization server respond with {err_response:?}"
                    )
                }
            }};
        }

        let response = loop {
            let response = self
                .client
                .post(Self::TOKEN_ENDPOINT)
                .header(http::header::ACCEPT, "application/json")
                .form(
                    &DeviceAccessTokenRequest::new(&device_code, self.client_id.as_ref())
                        .with_grant_type(Self::GRANT_TYPE)
                        .with_client_secret(self.client_secret.clone()),
                )
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
}
