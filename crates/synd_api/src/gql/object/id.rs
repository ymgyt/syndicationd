use std::convert::Infallible;

use async_graphql::connection::CursorType;
use synd_feed::types;

pub(crate) struct FeedIdV1(String);

impl FeedIdV1 {
    pub fn new(url: impl AsRef<str>) -> Self {
        let url = url.as_ref();
        Self(format!("v1:feed:{url}"))
    }
}

impl From<FeedIdV1> for async_graphql::ID {
    fn from(v: FeedIdV1) -> Self {
        Self(v.0)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::gql) struct EntryId<'a>(types::EntryId<'a>);

impl<'a> CursorType for EntryId<'a> {
    type Error = Infallible;

    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        let s = s.to_string();
        Ok(EntryId(s.into()))
    }

    fn encode_cursor(&self) -> String {
        self.0.to_string()
    }
}

impl<'a> From<types::EntryId<'a>> for EntryId<'a> {
    fn from(value: types::EntryId<'a>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_id_decode() {
        let id: EntryId = types::EntryId::from("123").into();

        assert_eq!(EntryId::decode_cursor(&id.encode_cursor()), Ok(id));
    }
}
