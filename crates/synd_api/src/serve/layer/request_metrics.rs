use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{extract::Request, response::Response};
use futures_util::Future;
use synd_o11y::metric;
use tower::{Layer, Service};

#[derive(Clone)]
pub struct RequestMetricsLayer {}

impl RequestMetricsLayer {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S> Layer<S> for RequestMetricsLayer {
    type Service = RequestMetricsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestMetricsService { inner }
    }
}

#[derive(Clone)]
pub struct RequestMetricsService<S> {
    inner: S,
}

impl<S> Service<Request> for RequestMetricsService<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = Infallible;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let path = req.uri().path().to_owned();

        let mut this = self.clone();
        Box::pin(async move {
            let response = this.inner.call(req).await.unwrap();
            let status = response.status().as_u16();

            metric!(monotonic_counter.request = 1, path, status);

            Ok(response)
        })
    }
}
