use serde::{Deserialize, Deserializer};

/// Validated continuation state of a GraphQL connection page.
#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub enum PageInfo {
    Complete { end_cursor: Option<String> },
    More { next_cursor: String },
}

impl<'de> Deserialize<'de> for PageInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PageInfoWire::deserialize(deserializer)?;
        match (wire.has_next_page, wire.end_cursor) {
            (false, end_cursor) => Ok(Self::Complete { end_cursor }),
            (true, Some(next_cursor)) => Ok(Self::More { next_cursor }),
            (true, None) => Err(serde::de::Error::custom(
                "page has a next page without an end cursor",
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfoWire {
    has_next_page: bool,
    end_cursor: Option<String>,
}
