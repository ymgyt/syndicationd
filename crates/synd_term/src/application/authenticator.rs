use synd_auth::{
    device_flow::{provider, DeviceFlow},
    jwt,
};

use crate::auth::{Credential, CredentialError};

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

    pub async fn refresh_google_id_token(
        &self,
        refresh_token: &str,
    ) -> Result<Credential, CredentialError> {
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
        Ok(credential)
    }
}

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
}
