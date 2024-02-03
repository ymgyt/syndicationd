use tower_http::trace::HttpMakeClassifier;
use tracing::Level;

#[derive(Clone)]
pub struct MakeSpan;

impl<B> tower_http::trace::MakeSpan<B> for MakeSpan {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> tracing::Span {
        use synd_o11y::opentelemetry::extension::*;
        let cx = synd_o11y::opentelemetry::http::extract(request.headers());

        let request_id = cx
            .baggage()
            .get("request.id")
            .map_or("?".into(), |v| v.as_str());

        let span = tracing::span!(
            Level::INFO,
            "http",
            method = %request.method(),
            uri = %request.uri(),
            %request_id,
        );

        tracing::info!("Request headers: {:#?}", request.headers());

        span.set_parent(cx);
        span
    }
}

#[derive(Clone)]
pub struct OnRequest;

impl<B> tower_http::trace::OnRequest<B> for OnRequest {
    fn on_request(&mut self, _request: &axum::http::Request<B>, _span: &tracing::Span) {
        // do nothing
    }
}

pub fn layer() -> tower_http::trace::TraceLayer<
    HttpMakeClassifier,
    MakeSpan,
    OnRequest,
    tower_http::trace::DefaultOnResponse,
> {
    tower_http::trace::TraceLayer::new_for_http()
        .make_span_with(MakeSpan)
        .on_request(OnRequest)
        .on_response(tower_http::trace::DefaultOnResponse::default().level(Level::INFO))
}
