use std::{
    ops::Sub,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use chrono::{DateTime, Utc};
use futures_util::{FutureExt as _, Stream};
use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_client::{ApiCredential, SyndApiError};
use tracing::{debug, info};
use url::Url;

use crate::{
    application::{Authenticator, FeedApiRef, JwtService, RequestError},
    auth::{AuthenticationProvider, Credential, Verified},
    config,
    event::{AuthEvent, Event},
};

use super::request::{RequestContext, RequestFuture};

type CredentialRefreshFuture = futures_util::future::BoxFuture<'static, Event>;

/// Executes authentication requests and owns the scheduled credential refresh.
pub(super) struct AuthDriver {
    pub(super) authenticator: Authenticator,
    api: FeedApiRef,
    credential_refresh: CredentialRefresh,
}

impl AuthDriver {
    pub(super) fn new(authenticator: Authenticator, api: FeedApiRef) -> Self {
        Self {
            authenticator,
            api,
            credential_refresh: CredentialRefresh::Inactive,
        }
    }

    pub(super) fn set_credential(
        &mut self,
        now: DateTime<Utc>,
        credential: &Verified<Credential>,
    ) -> Result<(), SyndApiError> {
        self.api.set_credential(credential.clone().into())?;
        self.credential_refresh
            .replace(&self.authenticator.jwt_service, now, credential);
        Ok(())
    }

    pub(super) fn start_device_flow(
        &self,
        provider: AuthenticationProvider,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let authenticator = self.authenticator.clone();

        move |context| {
            async move {
                info!("Start authenticate");
                let device_authorization = authenticator
                    .init_device_flow(provider)
                    .await
                    .map_err(RequestError::Authentication)?;
                let authorization = ValidatedDeviceAuthorization::try_from(device_authorization)
                    .map_err(RequestError::Authentication)?;

                context.emit_auth(authorization.into_event(provider));
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn poll_device_flow_access_token(
        &self,
        now: DateTime<Utc>,
        provider: AuthenticationProvider,
        device_authorization: DeviceAuthorizationResponse,
    ) -> impl FnOnce(RequestContext) -> RequestFuture + use<> {
        let authenticator = self.authenticator.clone();

        move |context| {
            async move {
                let credential = authenticator
                    .poll_device_flow_access_token(now, provider, device_authorization)
                    .await
                    .map_err(RequestError::Authentication)?;
                context.emit_auth(AuthEvent::DeviceFlowCredentialReceived { credential });
                Ok(())
            }
            .boxed()
        }
    }

    pub(super) fn stop(&mut self) {
        self.credential_refresh.stop();
    }
}

/// Scheduled credential refresh state owned by `AuthDriver`.
enum CredentialRefresh {
    Inactive,
    Scheduled { future: CredentialRefreshFuture },
}

impl CredentialRefresh {
    fn replace(
        &mut self,
        jwt_service: &JwtService,
        now: DateTime<Utc>,
        credential: &Verified<Credential>,
    ) {
        *self = match &**credential {
            Credential::Gh { .. } => Self::Inactive,
            Credential::Google {
                refresh_token,
                expired_at,
                ..
            } => Self::Scheduled {
                future: CredentialRefreshPlan::new(
                    jwt_service.clone(),
                    refresh_token.clone(),
                    now,
                    *expired_at,
                )
                .into_future(),
            },
        };
    }

    fn stop(&mut self) {
        *self = Self::Inactive;
    }
}

/// Inputs fixed when one Google credential refresh is scheduled.
struct CredentialRefreshPlan {
    jwt_service: JwtService,
    refresh_token: String,
    delay: Duration,
}

impl CredentialRefreshPlan {
    fn new(
        jwt_service: JwtService,
        refresh_token: String,
        now: DateTime<Utc>,
        expired_at: DateTime<Utc>,
    ) -> Self {
        let delay = expired_at
            .sub(config::credential::EXPIRE_MARGIN)
            .sub(now)
            .to_std()
            .unwrap_or(config::credential::FALLBACK_EXPIRE);

        Self {
            jwt_service,
            refresh_token,
            delay,
        }
    }

    fn into_future(self) -> CredentialRefreshFuture {
        async move {
            tokio::time::sleep(self.delay).await;

            debug!("Refresh google credential");
            match self
                .jwt_service
                .refresh_google_id_token(&self.refresh_token)
                .await
            {
                Ok(credential) => Event::CredentialRefreshed { credential },
                Err(error) => Event::CredentialRefreshFailed { error },
            }
        }
        .boxed()
    }
}

/// Device authorization response proven to contain a usable verification URL.
struct ValidatedDeviceAuthorization {
    verification_url: Url,
    response: DeviceAuthorizationResponse,
}

impl ValidatedDeviceAuthorization {
    fn into_event(self, provider: AuthenticationProvider) -> AuthEvent {
        AuthEvent::DeviceFlowAuthorizationReceived {
            provider,
            verification_url: self.verification_url,
            device_authorization: Box::new(self.response),
        }
    }
}

impl TryFrom<DeviceAuthorizationResponse> for ValidatedDeviceAuthorization {
    type Error = anyhow::Error;

    fn try_from(response: DeviceAuthorizationResponse) -> Result<Self, Self::Error> {
        let uri = response
            .verification_uri
            .as_ref()
            .or(response.verification_url.as_ref())
            .ok_or_else(|| {
                anyhow::anyhow!("device authorization response does not contain a verification URI")
            })?;
        let verification_url = Url::parse(uri.to_string().as_str())?;

        Ok(Self {
            verification_url,
            response,
        })
    }
}

impl Stream for AuthDriver {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Pin::new(&mut this.credential_refresh).poll_next(cx)
    }
}

impl Stream for CredentialRefresh {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let Self::Scheduled { future } = this else {
            return Poll::Pending;
        };

        match future.as_mut().poll(cx) {
            Poll::Ready(event) => {
                *this = Self::Inactive;
                Poll::Ready(Some(event))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl From<Verified<Credential>> for ApiCredential {
    fn from(credential: Verified<Credential>) -> Self {
        match credential.into_inner() {
            Credential::Gh { access_token } => Self::Github { access_token },
            Credential::Google { id_token, .. } => Self::Google { id_token },
        }
    }
}
