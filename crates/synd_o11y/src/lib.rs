use ::opentelemetry::KeyValue;

pub mod opentelemetry;
pub mod tracing_subscriber;

/// Request id key for opentelemetry baggage
pub const REQUEST_ID_KEY: &str = "request.id";

/// Generate random request id
pub fn request_id() -> String {
    // https://stackoverflow.com/questions/54275459/how-do-i-create-a-random-string-by-sampling-from-alphanumeric-characters
    use rand::distributions::{Alphanumeric, DistString};
    Alphanumeric.sample_string(&mut rand::thread_rng(), 10)
}

/// Generate random request id key value
pub fn request_id_key_value() -> KeyValue {
    KeyValue::new(REQUEST_ID_KEY, request_id())
}
