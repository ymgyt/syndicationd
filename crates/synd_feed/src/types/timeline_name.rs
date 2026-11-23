use std::fmt;

use serde::{Deserialize, Serialize};

use crate::macros::impl_sqlx_encode_decode;

/// Name given to a timeline by its owner.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String")]
pub struct TimelineName(String);

impl TimelineName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TimelineName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for TimelineName {
    fn from(name: &str) -> Self {
        Self::from(name.to_owned())
    }
}

impl fmt::Display for TimelineName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::Scalar]
impl async_graphql::ScalarType for TimelineName {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(name) => Ok(Self::from(name)),
            other => Err(async_graphql::InputValueError::expected_type(other)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.clone())
    }
}

impl_sqlx_encode_decode!(TimelineName as String);
