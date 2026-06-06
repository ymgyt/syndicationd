use std::ops::Sub;

use futures_util::FutureExt;
use synd_client::ApiCredential;
use tracing::{debug, info};

use crate::{
    application::RequestId,
    auth::{AuthenticationProvider, Credential, Verified},
    config,
    event::{ApiEvent, AuthApiEvent, Event},
};

use super::DriverContext;

pub(super) struct AuthDriver;

impl AuthDriver {
    pub(super) fn set_credential(cx: &mut DriverContext<'_>, cred: Verified<Credential>) {
        Self::schedule_credential_refreshing(cx, &cred);
        cx.adapters
            .feed_api
            .set_credential(cred.into())
            .expect("credential value must be a valid HTTP header");
    }

    pub(super) fn start_device_flow(
        cx: &mut DriverContext<'_>,
        provider: AuthenticationProvider,
    ) -> Vec<Event> {
        info!("Start authenticate");

        let authenticator = cx.adapters.authenticator.clone();
        let request_seq = cx
            .runtime
            .request_started(RequestId::DeviceFlowDeviceAuthorize);
        let fut = async move {
            match authenticator.init_device_flow(provider).await {
                Ok(device_authorization) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Auth(AuthApiEvent::DeviceFlowAuthorizationReceived {
                        provider,
                        device_authorization: Box::new(device_authorization),
                    }),
                }),
                Err(err) => Ok(Event::oauth_api_error(err, request_seq)),
            }
        }
        .boxed();
        cx.runtime.push_job(fut);
        Vec::new()
    }

    pub(super) fn poll_device_flow_access_token(
        cx: &mut DriverContext<'_>,
        provider: AuthenticationProvider,
        device_authorization: synd_auth::device_flow::DeviceAuthorizationResponse,
    ) -> Vec<Event> {
        let authenticator = cx.adapters.authenticator.clone();
        let now = cx.adapters.now();
        let request_seq = cx
            .runtime
            .request_started(RequestId::DeviceFlowPollAccessToken);
        let fut = async move {
            match authenticator
                .poll_device_flow_access_token(now, provider, device_authorization)
                .await
            {
                Ok(credential) => Ok(Event::Api {
                    request_seq,
                    event: ApiEvent::Auth(AuthApiEvent::DeviceFlowCredentialReceived {
                        credential,
                    }),
                }),
                Err(err) => Ok(Event::oauth_api_error(err, request_seq)),
            }
        }
        .boxed();

        cx.runtime.push_job(fut);
        Vec::new()
    }

    fn schedule_credential_refreshing(cx: &mut DriverContext<'_>, cred: &Verified<Credential>) {
        match &**cred {
            Credential::Github { .. } => {}
            Credential::Google {
                refresh_token,
                expired_at,
                ..
            } => {
                let until_expire = expired_at
                    .sub(config::credential::EXPIRE_MARGIN)
                    .sub(cx.adapters.now())
                    .to_std()
                    .unwrap_or(config::credential::FALLBACK_EXPIRE);
                let jwt_service = cx.adapters.authenticator.jwt_service.clone();
                let refresh_token = refresh_token.clone();
                let fut = async move {
                    tokio::time::sleep(until_expire).await;

                    debug!("Refresh google credential");
                    match jwt_service.refresh_google_id_token(&refresh_token).await {
                        Ok(credential) => Ok(Event::CredentialRefreshed { credential }),
                        Err(err) => Ok(Event::Error {
                            message: err.to_string(),
                        }),
                    }
                }
                .boxed();
                cx.runtime.push_background_job(fut);
            }
        }
    }
}

impl From<Verified<Credential>> for ApiCredential {
    fn from(cred: Verified<Credential>) -> Self {
        match cred.into_inner() {
            Credential::Github { access_token } => Self::Github { access_token },
            Credential::Google { id_token, .. } => Self::Google { id_token },
        }
    }
}
