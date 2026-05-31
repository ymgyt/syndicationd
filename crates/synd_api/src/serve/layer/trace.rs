use std::time::Duration;

use tower_http::trace::HttpMakeClassifier;
use tracing::{Level, Span};
use tracing::{debug, field, info, span, warn};

const SLOW_RESPONSE_THRESHOLD: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct MakeSpan;

impl<B> tower_http::trace::MakeSpan<B> for MakeSpan {
    #[expect(clippy::redundant_closure_for_method_calls)]
    fn make_span(&mut self, request: &axum::http::Request<B>) -> Span {
        use synd_support::o11y::opentelemetry::extension::*;
        let cx = synd_support::o11y::opentelemetry::http::extract(request.headers());

        let request_id = cx
            .baggage()
            .get(synd_support::o11y::REQUEST_ID_KEY)
            .map(|v| v.as_str());

        let span = span!(
            Level::INFO,
            "http",
            method = %request.method(),
            path = %request.uri().path(),
            request_id = field::Empty,
        );
        if let Some(request_id) = request_id {
            span.record("request_id", request_id);
        }

        let _ = span.set_parent(cx);
        span
    }
}

#[derive(Clone)]
pub struct OnRequest;

impl<B> tower_http::trace::OnRequest<B> for OnRequest {
    fn on_request(&mut self, _request: &axum::http::Request<B>, _span: &Span) {
        // do nothing
    }
}

#[derive(Clone)]
pub struct LogResponse;

impl<B> tower_http::trace::OnResponse<B> for LogResponse {
    fn on_response(self, response: &axum::http::Response<B>, latency: Duration, _span: &Span) {
        let status = response.status();
        let latency_ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);

        if status.is_server_error() {
            warn!(latency_ms, status = status.as_u16(), "request failed");
        } else if status.is_client_error() {
            info!(latency_ms, status = status.as_u16(), "request rejected");
        } else if latency >= SLOW_RESPONSE_THRESHOLD {
            info!(latency_ms, status = status.as_u16(), "slow request");
        } else {
            debug!(latency_ms, status = status.as_u16(), "request complete");
        }
    }
}

pub fn layer() -> tower_http::trace::TraceLayer<HttpMakeClassifier, MakeSpan, OnRequest, LogResponse>
{
    tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(MakeSpan)
        .on_request(OnRequest)
        .on_response(LogResponse)
}
