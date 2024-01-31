use async_graphql::{InputValueError, Scalar, ScalarType, Value};
use chrono::Utc;

/// RFC3339 Time
pub struct Rfc3339Time(synd::types::Time);

#[Scalar]
impl ScalarType for Rfc3339Time {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        let Value::String(value) = value else {
            return Err(InputValueError::expected_type(value));
        };

        chrono::DateTime::parse_from_rfc3339(&value)
            .map(|t| t.with_timezone(&Utc))
            .map(Rfc3339Time)
            .map_err(InputValueError::custom)
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_rfc3339())
    }
}

impl From<synd::types::Time> for Rfc3339Time {
    fn from(value: synd::types::Time) -> Self {
        Self(value)
    }
}
