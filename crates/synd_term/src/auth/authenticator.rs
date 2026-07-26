use std::ops::Add;
use synd_auth::{
    device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse, DeviceFlow, provider},
    jwt,
};

use crate::{
    auth::{AuthenticationProvider, Credential, CredentialError, Verified},
    config,
    types::Time,
};

#[derive(Clone)]
pub struct DeviceFlows {
    pub gh: DeviceFlow<provider::Github>,
    pub google: DeviceFlow<provider::Google>,
}

#[derive(Clone)]
pub struct JwtService {
    pub google: jwt::google::JwtService,
}

impl JwtService {
    pub fn new() -> Self {
        Self {
            google: jwt::google::JwtService::default(),
        }
    }

    #[must_use]
    pub fn with_google_jwt_service(self, google: jwt::google::JwtService) -> Self {
        Self { google }
    }

    pub(crate) async fn refresh_google_id_token(
        &self,
        refresh_token: &str,
    ) -> Result<Verified<Credential>, CredentialError> {
        let id_token = self
            .google
            .refresh_id_token(refresh_token)
            .await
            .map_err(CredentialError::RefreshJwt)?;
        let expired_at = self
            .google
            .decode_id_token_insecure(&id_token, false)
            .map_err(CredentialError::DecodeJwt)?
            .expired_at();
        let credential = Credential::Google {
            id_token,
            refresh_token: refresh_token.to_owned(),
            expired_at,
        };
        Ok(Verified(credential))
    }
}

#[derive(Clone)]
pub struct Authenticator {
    pub device_flows: DeviceFlows,
    pub jwt_service: JwtService,
}

impl Authenticator {
    pub fn new() -> Self {
        Self {
            device_flows: DeviceFlows {
                gh: DeviceFlow::new(provider::Github::default()),
                google: DeviceFlow::new(provider::Google::default()),
            },
            jwt_service: JwtService::new(),
        }
    }

    #[must_use]
    pub fn with_device_flows(self, device_flows: DeviceFlows) -> Self {
        Self {
            device_flows,
            ..self
        }
    }

    #[must_use]
    pub fn with_jwt_service(self, jwt_service: JwtService) -> Self {
        Self {
            jwt_service,
            ..self
        }
    }

    pub(crate) async fn init_device_flow(
        &self,
        provider: AuthenticationProvider,
    ) -> anyhow::Result<DeviceAuthorizationResponse> {
        match provider {
            AuthenticationProvider::Gh => self.device_flows.gh.device_authorize_request().await,

            AuthenticationProvider::Google => {
                self.device_flows.google.device_authorize_request().await
            }
        }
    }

    pub(crate) async fn poll_device_flow_access_token(
        &self,
        now: Time,
        provider: AuthenticationProvider,
        response: DeviceAuthorizationResponse,
    ) -> anyhow::Result<Verified<Credential>> {
        match provider {
            AuthenticationProvider::Gh => {
                let token_response = self
                    .device_flows
                    .gh
                    .poll_device_access_token(response.device_code, response.interval)
                    .await?;

                Ok(Verified(Credential::Gh {
                    access_token: token_response.access_token,
                }))
            }
            AuthenticationProvider::Google => {
                let token_response = self
                    .device_flows
                    .google
                    .poll_device_access_token(response.device_code, response.interval)
                    .await?;

                let tokens = GoogleDeviceFlowTokens::try_from(token_response)?;
                Ok(tokens.into_credential(&self.jwt_service, now))
            }
        }
    }
}

/// Google device-flow response proven to contain both OIDC tokens.
struct GoogleDeviceFlowTokens {
    id_token: String,
    refresh_token: String,
}

impl GoogleDeviceFlowTokens {
    fn into_credential(self, jwt_service: &JwtService, now: Time) -> Verified<Credential> {
        let expired_at = jwt_service
            .google
            .decode_id_token_insecure(&self.id_token, false)
            .ok()
            .map_or(now.add(config::credential::FALLBACK_EXPIRE), |claims| {
                claims.expired_at()
            });

        Verified(Credential::Google {
            id_token: self.id_token,
            refresh_token: self.refresh_token,
            expired_at,
        })
    }
}

impl TryFrom<DeviceAccessTokenResponse> for GoogleDeviceFlowTokens {
    type Error = anyhow::Error;

    fn try_from(response: DeviceAccessTokenResponse) -> Result<Self, Self::Error> {
        let id_token = response
            .id_token
            .ok_or_else(|| anyhow::anyhow!("Google device flow response has no ID token"))?;
        let refresh_token = response
            .refresh_token
            .ok_or_else(|| anyhow::anyhow!("Google device flow response has no refresh token"))?;

        Ok(Self {
            id_token,
            refresh_token,
        })
    }
}
