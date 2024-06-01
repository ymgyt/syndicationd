#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[cfg_attr(faeture = "fake", derive(fake::Dummy))]
pub enum FeedType {
    Atom,
    #[allow(clippy::upper_case_acronyms)]
    JSON,
    RSS0,
    RSS1,
    RSS2,
}

impl From<feed_rs::model::FeedType> for FeedType {
    fn from(typ: feed_rs::model::FeedType) -> Self {
        match typ {
            feed_rs::model::FeedType::Atom => FeedType::Atom,
            feed_rs::model::FeedType::JSON => FeedType::JSON,
            feed_rs::model::FeedType::RSS0 => FeedType::RSS0,
            feed_rs::model::FeedType::RSS1 => FeedType::RSS1,
            feed_rs::model::FeedType::RSS2 => FeedType::RSS2,
        }
    }
}
