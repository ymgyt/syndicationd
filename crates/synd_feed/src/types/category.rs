use std::{
    borrow::Cow,
    fmt::{self, Display},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CategoryError {
    #[error("not empty validation is violated")]
    NotEmptyViolated,
    #[error("len max validation is violated")]
    LenMaxViolated,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub struct Category<'a>(Cow<'a, str>);

impl<'a> Category<'a> {
    const MAX_LEN: usize = 30;
    pub fn new(c: impl Into<Cow<'a, str>>) -> Result<Self, CategoryError> {
        let c = c.into().trim().to_ascii_lowercase();

        match c.len() {
            0 => return Err(CategoryError::NotEmptyViolated),
            n if n > Self::MAX_LEN => return Err(CategoryError::LenMaxViolated),
            _ => {}
        }

        Ok(Self(c.into()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn into_inner(self) -> Cow<'a, str> {
        self.0
    }
}

impl<'s> TryFrom<&'s str> for Category<'s> {
    type Error = CategoryError;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Display for Category<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.0.as_ref())
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::Scalar]
impl async_graphql::ScalarType for Category<'_> {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        let async_graphql::Value::String(s) = value else {
            return Err(async_graphql::InputValueError::expected_type(value));
        };

        match Category::new(s) {
            Ok(c) => Ok(c),
            Err(err) => Err(async_graphql::InputValueError::custom(err)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        // Is this clone avoidable?
        async_graphql::Value::String(self.0.clone().into_owned())
    }
}

#[cfg(feature = "fake")]
impl fake::Dummy<fake::Faker> for Category<'_> {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &fake::Faker, rng: &mut R) -> Self {
        let category: String = fake::Fake::fake_with_rng(&(1..31), rng);
        Self::new(category).unwrap()
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Sqlite> for Category<'static> {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for Category<'static> {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        self.to_string().encode_by_ref(buf)
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Category<'static> {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Self::new(Cow::Owned(s)).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_spec() {
        assert_eq!(Category::new(""), Err(CategoryError::NotEmptyViolated));
        assert_eq!(
            Category::new("a".repeat(Category::MAX_LEN + 1)),
            Err(CategoryError::LenMaxViolated)
        );

        assert!(Category::new("a".repeat(Category::MAX_LEN) + "  ").is_ok(),);

        assert_eq!(
            Category::new("rust").unwrap().into_inner(),
            format!("{}", Category::new("rust").unwrap()),
        );
    }

    #[test]
    #[cfg(feature = "graphql")]
    fn scalar() {
        use async_graphql::ScalarType;

        assert!(Category::parse(async_graphql::Value::Null).is_err());
        assert!(
            Category::parse(async_graphql::Value::String(
                "a".repeat(Category::MAX_LEN + 1)
            ))
            .is_err()
        );
    }

    #[test]
    #[cfg(feature = "fake")]
    fn fake() {
        use fake::Fake;

        let c: Category = fake::Faker.fake();
        assert!(!c.as_str().is_empty());
    }
}
