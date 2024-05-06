//! RFC Draft Health Check Response Format for HTTP APIs implementation
//! [RFC Draft](https://datatracker.ietf.org/doc/html/draft-inadarei-api-health-check)

use core::fmt;
use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Indicates whether the service status is acceptable or not.
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// healthy
    #[default]
    Pass,
    /// Unhealthy
    Fail,
    /// healthy, with some concerns
    Warn,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pass => f.write_str("pass"),
            Status::Fail => f.write_str("fail"),
            Status::Warn => f.write_str("warn"),
        }
    }
}

/// Represents Api Health
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Health {
    pub status: Status,
    pub version: Option<Cow<'static, str>>,
    pub description: Option<Cow<'static, str>>,
}

impl Health {
    /// [RFC API Health Response](https://datatracker.ietf.org/doc/html/draft-inadarei-api-health-check#name-api-health-response)
    pub const CONTENT_TYPE: &'static str = "application/health+json";

    pub fn pass() -> Self {
        Self {
            status: Status::Pass,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_version(self, version: impl Into<Cow<'static, str>>) -> Self {
        Self {
            version: Some(version.into()),
            ..self
        }
    }

    #[must_use]
    pub fn with_description(self, description: impl Into<Cow<'static, str>>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }
}
