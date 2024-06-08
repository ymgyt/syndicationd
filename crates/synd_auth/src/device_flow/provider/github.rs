use std::borrow::Cow;

use reqwest::Url;

use crate::{
    config,
    device_flow::{DeviceAccessTokenRequest, DeviceAuthorizationRequest, Provider},
};

#[derive(Clone)]
pub struct Github {
    client_id: Cow<'static, str>,
    device_authorization_endpoint: Url,
    token_endpoint: Url,
}

impl Default for Github {
    fn default() -> Self {
        Self::new(config::github::CLIENT_ID)
    }
}

impl Github {
    const DEVICE_AUTHORIZATION_ENDPOINT: &'static str = "https://github.com/login/device/code";
    const TOKEN_ENDPOINT: &'static str = "https://github.com/login/oauth/access_token";
    // https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
    const SCOPE: &'static str = "user:email";

    pub fn new(client_id: impl Into<Cow<'static, str>>) -> Self {
        Self {
            client_id: client_id.into(),
            device_authorization_endpoint: Url::parse(Self::DEVICE_AUTHORIZATION_ENDPOINT).unwrap(),
            token_endpoint: Url::parse(Self::TOKEN_ENDPOINT).unwrap(),
        }
    }

    #[must_use]
    pub fn with_device_authorization_endpoint(self, endpoint: Url) -> Self {
        Self {
            device_authorization_endpoint: endpoint,
            ..self
        }
    }

    #[must_use]
    pub fn with_token_endpoint(self, endpoint: Url) -> Self {
        Self {
            token_endpoint: endpoint,
            ..self
        }
    }
}

impl Provider for Github {
    type DeviceAccessTokenRequest<'d> = DeviceAccessTokenRequest<'d>;
    fn device_authorization_endpoint(&self) -> Url {
        self.device_authorization_endpoint.clone()
    }

    fn token_endpoint(&self) -> reqwest::Url {
        self.token_endpoint.clone()
    }

    fn device_authorization_request(&self) -> DeviceAuthorizationRequest {
        DeviceAuthorizationRequest {
            client_id: self.client_id.clone(),
            scope: Self::SCOPE.into(),
        }
    }

    fn device_access_token_request<'d, 'p: 'd>(
        &'p self,
        device_code: &'d str,
    ) -> DeviceAccessTokenRequest<'d> {
        DeviceAccessTokenRequest::new(device_code, self.client_id.as_ref())
    }
}
