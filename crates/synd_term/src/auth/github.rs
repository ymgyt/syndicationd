use std::{io::Write, time::Duration};

use http::StatusCode;
use reqwest::Client;
use tracing::debug;

use crate::{
    auth::device_flow::{
        DeviceAccessTokenErrorResponse, DeviceAccessTokenRequest, DeviceAccessTokenResponse,
        DeviceAuthorizationRequest, DeviceAuthorizationResponse,
    },
    config,
};

/// https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow
#[derive(Clone)]
pub struct DeviceFlow {
    client: Client,
    client_id: &'static str,
    endpoint: Option<&'static str>,
}

impl DeviceFlow {
    const DEVICE_AUTHORIZATION_ENDPOINT: &'static str = "https://github.com/login/device/code";
    const TOKEN_ENDPOINT: &'static str = "https://github.com/login/oauth/access_token";

    pub fn new() -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(config::USER_AGENT)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        Self {
            client,
            client_id: config::github::CLIENT_ID,
            endpoint: None,
        }
    }

    pub fn with_endpoint(self, endpoint: &'static str) -> Self {
        Self {
            endpoint: Some(endpoint),
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
            .post(self.endpoint.unwrap_or(Self::DEVICE_AUTHORIZATION_ENDPOINT))
            .header(http::header::ACCEPT, "application/json")
            .form(&DeviceAuthorizationRequest {
                client_id: self.client_id.into(),
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
                .post(Self::TOKEN_ENDPOINT)
                .header(http::header::ACCEPT, "application/json")
                .form(&DeviceAccessTokenRequest::new(&device_code, self.client_id))
                .send()
                .await?;

            debug!("{:?}", response.status());

            match response.status() {
                StatusCode::OK => {
                    let full = response.bytes().await?;
                    match serde_json::from_slice::<DeviceAccessTokenResponse>(&full) {
                        Ok(response) => break response,
                        Err(_) => continue_or_abort!(full),
                    }
                }
                StatusCode::BAD_REQUEST => {
                    let full = response.bytes().await?;
                    continue_or_abort!(full)
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
    pub async fn device_flow<W: Write>(
        self,
        writer: W,
    ) -> anyhow::Result<DeviceAccessTokenResponse> {
        let DeviceAuthorizationResponse {
            device_code,
            user_code,
            verification_uri,
            interval,
            ..
        } = self.device_authorize_request().await?;

        let mut writer = writer;
        writeln!(&mut writer, "Open `{verification_uri}` on your browser")?;
        writeln!(&mut writer, "Enter CODE: `{user_code}`")?;

        // attempt to open input screen in the browser
        open::that(verification_uri.to_string()).ok();

        self.pool_device_access_token(device_code, interval).await
    }
}
