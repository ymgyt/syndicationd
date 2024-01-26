use std::task::{Context, Poll};

use axum::http::{self, StatusCode};
use axum::response::IntoResponse;
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use crate::serve::auth::Authenticator;

#[derive(Clone)]
pub struct AuthenticateLayer {
    authenticator: Authenticator,
}

impl AuthenticateLayer {
    pub fn new(authenticator: Authenticator) -> Self {
        Self { authenticator }
    }
}

impl<S> Layer<S> for AuthenticateLayer {
    type Service = AuthenticateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthenticateService {
            inner,
            authenticator: self.authenticator.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthenticateService<S> {
    authenticator: Authenticator,
    inner: S,
}

impl<S> Service<axum::extract::Request> for AuthenticateService<S>
where
    S: Service<axum::extract::Request, Response = axum::response::Response>
        + Send
        + 'static
        + Clone,
    S::Future: Send + 'static,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: axum::extract::Request) -> Self::Future {
        let header = request
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok());

        let Some(token) = header else {
            return Box::pin(async { Ok(StatusCode::UNAUTHORIZED.into_response()) });
        };

        let Self {
            authenticator,
            mut inner,
        } = self.clone();
        let token = token.to_owned();

        Box::pin(async move {
            let principal = match authenticator.authenticate(token).await {
                Ok(principal) => principal,
                Err(_) => {
                    tracing::warn!("Invalid token");
                    return Ok(StatusCode::UNAUTHORIZED.into_response());
                }
            };

            request.extensions_mut().insert(principal);

            inner.call(request).await
        })
    }
}
