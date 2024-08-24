use std::{borrow::Borrow, fmt};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum FeedUrlError {
    #[error("invalid url: {0}")]
    InvalidUrl(url::ParseError),
}

/// Feed Url which serve rss or atom
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub struct FeedUrl(Url);

impl Borrow<Url> for FeedUrl {
    fn borrow(&self) -> &Url {
        &self.0
    }
}

impl TryFrom<&str> for FeedUrl {
    type Error = FeedUrlError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Url::parse(s).map(FeedUrl).map_err(FeedUrlError::InvalidUrl)
    }
}

impl AsRef<str> for FeedUrl {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<Url> for FeedUrl {
    fn from(url: Url) -> Self {
        Self(url)
    }
}

impl From<FeedUrl> for Url {
    fn from(url: FeedUrl) -> Self {
        url.0
    }
}

impl fmt::Display for FeedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FeedUrl {
    pub fn into_inner(self) -> Url {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn parse(input: &str) -> Result<Self, url::ParseError> {
        Url::parse(input).map(FeedUrl)
    }
}

#[cfg(feature = "graphql")]
#[async_graphql::Scalar]
impl async_graphql::ScalarType for FeedUrl {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        let async_graphql::Value::String(s) = value else {
            return Err(async_graphql::InputValueError::expected_type(value));
        };

        match Url::parse(&s) {
            Ok(url) => Ok(FeedUrl::from(url)),
            Err(err) => Err(async_graphql::InputValueError::custom(err)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        // Is this clone avoidable?
        async_graphql::Value::String(self.0.clone().into())
    }
}

#[cfg(feature = "fake")]
impl fake::Dummy<fake::Faker> for FeedUrl {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &fake::Faker, _rng: &mut R) -> Self {
        Url::parse("https://fake.ymgyt.io").unwrap().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backward_compatible() {
        let org = "https://blog.ymgyt.io/atom.xml";
        let u = FeedUrl::parse(org).unwrap();

        assert_eq!(u.as_str(), org);
        assert_eq!(format!("{u}").as_str(), org);
    }

    #[test]
    fn deserialize_from_strings() {
        let data = vec![
            "https://blog.ymgyt.io/atom.xml",
            "https://blog.ymgyt.io/atom2.xml",
        ];
        let serialized = serde_json::to_string(&data).unwrap();
        let deserialized: Vec<FeedUrl> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized,
            vec![
                FeedUrl::parse("https://blog.ymgyt.io/atom.xml").unwrap(),
                FeedUrl::parse("https://blog.ymgyt.io/atom2.xml").unwrap(),
            ],
        );
    }

    #[test]
    fn url() {
        let u = FeedUrl::parse("https://blog.ymgyt.io/atom.xml").unwrap();

        assert_eq!(Borrow::<Url>::borrow(&u), &Url::from(u.clone()));
    }

    #[test]
    #[cfg(feature = "graphql")]
    fn scalar() {
        use async_graphql::ScalarType;

        assert!(<FeedUrl as ScalarType>::parse(async_graphql::Value::Null).is_err());
        assert!(
            <FeedUrl as ScalarType>::parse(async_graphql::Value::String("invalid".into())).is_err()
        );
    }
}
