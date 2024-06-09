use async_graphql::{InputValueError, Scalar, ScalarType, Value};
use chrono::Utc;

/// RFC3339 Time
pub struct Rfc3339Time(synd_feed::types::Time);

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

impl From<synd_feed::types::Time> for Rfc3339Time {
    fn from(value: synd_feed::types::Time) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rfc3339_time() {
        assert!(Rfc3339Time::parse(async_graphql::Value::Null).is_err());
        assert!(
            Rfc3339Time::parse(async_graphql::Value::String("2024-06-09T01:02:03Z".into())).is_ok()
        );
    }
}
