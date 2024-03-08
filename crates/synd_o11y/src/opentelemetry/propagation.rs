use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};

pub mod extension {
    pub use opentelemetry::baggage::BaggageExt as _;
    pub use tracing_opentelemetry::OpenTelemetrySpanExt as _;
}

/// Currently axum and reqwest have different http crate versions.
/// axum is ver 1, reqwest ver 0.2, therefore, we use each type in inject and extract.
pub mod http {
    use super::extension::*;
    use opentelemetry::propagation::Extractor;
    use opentelemetry_http::HeaderInjector;

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

    /// `opentelemetry_http` implement `HeaderExtractor` against http 0.2
    /// so, imple manually
    struct HeaderExtractor<'a>(pub &'a axum::http::HeaderMap);

    impl<'a> Extractor for HeaderExtractor<'a> {
        /// Get a value for a key from the `HeaderMap`.  If the value is not valid ASCII, returns None.
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|value| value.to_str().ok())
        }

        /// Collect all the keys from the `HeaderMap`.
        fn keys(&self) -> Vec<&str> {
            self.0
                .keys()
                .map(axum::http::HeaderName::as_str)
                .collect::<Vec<_>>()
        }
    }
}

pub fn init_propagation() {
    let trace_propagator = TraceContextPropagator::new();
    let baggage_propagator = BaggagePropagator::new();
    let composite_propagator = TextMapCompositePropagator::new(vec![
        Box::new(trace_propagator),
        Box::new(baggage_propagator),
    ]);

    opentelemetry::global::set_text_map_propagator(composite_propagator);
}
