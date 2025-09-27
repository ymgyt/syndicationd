use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use pin_project::pin_project;
use tower::{Layer, Service};

use crate::principal::Principal;

pub trait Authenticate {
    // how to implementor fill this associate type
    // need impl trait in associate type ?
    // https://github.com/rust-lang/rust/issues/63063
    type Output: Future<Output = Result<Principal, ()>>;

    fn authenticate(&self, token: Option<String>) -> Self::Output;
}

#[expect(clippy::large_enum_variant)]
#[pin_project(project = AuthFutureProj)]
pub enum AuthenticateFuture<AuthFut, S, F> {
    Authenticate {
        req: Option<Request>,
        #[pin]
        auth_fut: AuthFut,
        inner: S,
    },
    ServiceCall {
        #[pin]
        service_fut: F,
    },
}

impl<AuthFut, S, F> AuthenticateFuture<AuthFut, S, F> {
    fn new(req: Request, auth_fut: AuthFut, inner: S) -> Self {
        AuthenticateFuture::Authenticate {
            req: Some(req),
            auth_fut,
            inner,
        }
    }
}

impl<AuthFut, S> Future for AuthenticateFuture<AuthFut, S, S::Future>
where
    AuthFut: Future<Output = Result<Principal, ()>>,
    S: Service<Request, Response = Response, Error = Infallible>,
{
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            AuthFutureProj::Authenticate {
                req,
                auth_fut,
                inner,
            } => match auth_fut.poll(cx) {
                Poll::Ready(Ok(principal)) => {
                    let mut req = req.take().unwrap();
                    req.extensions_mut().insert(principal);
                    let service_fut = inner.call(req);

                    self.set(AuthenticateFuture::ServiceCall { service_fut });
                    self.poll(cx)
                }
                Poll::Ready(Err(())) => Poll::Ready(Ok(StatusCode::UNAUTHORIZED.into_response())),
                Poll::Pending => Poll::Pending,
            },
            AuthFutureProj::ServiceCall { service_fut } => service_fut.poll(cx),
        }
    }
}

#[derive(Clone)]
pub struct AuthenticateLayer<A> {
    authenticator: A,
}

impl<A> AuthenticateLayer<A> {
    pub fn new(authenticator: A) -> Self {
        Self { authenticator }
    }
}

impl<S, A> Layer<S> for AuthenticateLayer<A>
where
    A: Authenticate + Clone,
{
    type Service = AuthenticateService<S, A>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthenticateService {
            inner,
            authenticator: self.authenticator.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AuthenticateService<S, A> {
    inner: S,
    authenticator: A,
}

impl<S, A> Service<Request> for AuthenticateService<S, A>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone,
    A: Authenticate,
{
    type Response = Response;
    type Error = Infallible;
    type Future = AuthenticateFuture<A::Output, S, S::Future>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let token = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .map(ToOwned::to_owned);

        let auth_fut = self.authenticator.authenticate(token);
        let inner = self.inner.clone();

        AuthenticateFuture::new(req, auth_fut, inner)
    }
}
