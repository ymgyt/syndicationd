use std::time::Duration;

pub mod service;

/// Polling interval used for a new subscription when no interval is specified.
pub const DEFAULT_POLLING_INTERVAL: Duration = Duration::from_hours(2);
