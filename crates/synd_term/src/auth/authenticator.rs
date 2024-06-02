use std::ops::Add;
use synd_auth::{
    device_flow::{provider, DeviceAuthorizationResponse, DeviceFlow},
    jwt,
};

use crate::{
    auth::{AuthenticationProvider, Credential, CredentialError, Verified},
    config,
    types::Time,
};

#[derive(Clone)]
pub struct DeviceFlows {
    pub github: DeviceFlow<provider::Github>,
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
                github: DeviceFlow::new(provider::Github::default()),
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

    pub(crate) async fn init_device_flow(
        &self,
        provider: AuthenticationProvider,
    ) -> anyhow::Result<DeviceAuthorizationResponse> {
        match provider {
            AuthenticationProvider::Github => {
                self.device_flows.github.device_authorize_request().await
            }

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
            AuthenticationProvider::Github => {
                let token_response = self
                    .device_flows
                    .github
                    .poll_device_access_token(response.device_code, response.interval)
                    .await?;

                Ok(Verified(Credential::Github {
                    access_token: token_response.access_token,
                }))
            }
            AuthenticationProvider::Google => {
                let token_response = self
                    .device_flows
                    .google
                    .poll_device_access_token(response.device_code, response.interval)
                    .await?;

                let id_token = token_response.id_token.expect("id token not found");
                let expired_at = self
                    .jwt_service
                    .google
                    .decode_id_token_insecure(&id_token, false)
                    .ok()
                    .map_or(now.add(config::credential::FALLBACK_EXPIRE), |claims| {
                        claims.expired_at()
                    });
                Ok(Verified(Credential::Google {
                    id_token,
                    refresh_token: token_response
                        .refresh_token
                        .expect("refresh token not found"),
                    expired_at,
                }))
            }
        }
    }
}
