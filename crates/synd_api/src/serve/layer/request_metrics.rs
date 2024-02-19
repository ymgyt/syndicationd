use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use axum::{extract::Request, http::Method, response::Response};
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
        let start = Instant::now();
        let path = req.uri().path().to_owned();
        let method = req.method().clone();

        let mut this = self.clone();
        Box::pin(async move {
            let response = this.inner.call(req).await.unwrap();
            let status = response.status().as_u16();

            // https://opentelemetry.io/docs/specs/semconv/http/http-metrics/
            // Considiering the case of not found(404), recording the path as
            // an attribute leads to an inability to control cardinality.
            // Therefore, the path is not recorded.
            metric!(
                monotonic_counter.http.server.request = 1,
                http.response.status.code = status
            );

            // instrument graphql latency
            if path == "/graphql" && method == Method::POST {
                // f64 is matter
                // The type of the field that MetricsVisitor visits when on_event() is called is pre defined.
                // If u128 which is returned from elapsed() is used, it will not be visited, resulting in no metrics recorded.
                // Spec say "When instruments are measuring durations, seconds SHOULD be used"
                // https://opentelemetry.io/docs/specs/semconv/general/metrics/#instrument-units
                let elapsed: f64 = start.elapsed().as_secs_f64();
                // is there any semantic conventions?
                metric!(histogram.graphql.duration = elapsed);
            }

            Ok(response)
        })
    }
}
