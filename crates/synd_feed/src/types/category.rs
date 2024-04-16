use std::{
    borrow::Cow,
    fmt::{self, Display},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CategoryError {
    #[error("not empty validation is violated")]
    NotEmptyViolated,
    #[error("len max validation is violated")]
    LenMaxViolated,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl<'a> Display for Category<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::Scalar]
impl<'s> async_graphql::ScalarType for Category<'s> {
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