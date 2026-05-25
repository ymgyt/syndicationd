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
