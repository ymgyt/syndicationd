use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};

/// Currently axum and reqwest have different http crate versions.
/// axum is ver 1, reqwest ver 0.2, therefore, we use each type in inject and extract.
pub mod http {
    use crate::opentelemetry::extension::*;
    use opentelemetry_http::{HeaderExtractor, HeaderInjector};

    /// Inject current opentelemetry context into given headers
    pub fn inject(cx: &opentelemetry::Context, headers: &mut reqwest::header::HeaderMap) {
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject_context(cx, &mut HeaderInjector(headers));
        });
    }
    pub fn inject_with_baggage<T, I>(
        cx: &opentelemetry::Context,
        headers: &mut reqwest::header::HeaderMap,
        baggage: T,
    ) where
        T: IntoIterator<Item = I>,
        I: Into<opentelemetry::baggage::KeyValueMetadata>,
    {
        inject(&cx.with_baggage(baggage), headers);
    }

    pub fn extract(headers: &axum::http::HeaderMap) -> opentelemetry::Context {
        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(headers))
        })
    }
}

/// Configure `TraceContext` and Baggage propagator then set as global propagator
pub fn init_propagation() {
    let trace_propagator = TraceContextPropagator::new();
    let baggage_propagator = BaggagePropagator::new();
    let composite_propagator = TextMapCompositePropagator::new(vec![
        Box::new(trace_propagator),
        Box::new(baggage_propagator),
    ]);

    opentelemetry::global::set_text_map_propagator(composite_propagator);
}
