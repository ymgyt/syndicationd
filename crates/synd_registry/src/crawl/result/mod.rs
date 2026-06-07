mod detail;
mod record;
mod state;

pub use detail::{
    CrawlFeedParseErrorDetail, CrawlFetchErrorDetail, CrawlHttpBodyDetail, CrawlHttpResponseDetail,
    CrawlResultDetail,
};
pub use record::{CrawlResultRecord, CrawlResultRef, RecordCrawlResultCommand};
pub use state::{
    CrawlHealth, CrawlHttpErrorKind, CrawlState, CrawlStateError, CrawlStateErrorKind,
    CrawlStateTimestamps, FailureStreak, LastCrawlResult, UpsertCrawlStateCommand,
};
