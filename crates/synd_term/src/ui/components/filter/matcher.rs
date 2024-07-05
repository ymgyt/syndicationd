use crate::{
    matcher::Matcher,
    types::{self, github::Notification},
    ui::components::filter::{FilterResult, Filterable},
};

#[derive(Default, Clone, Debug)]
pub(crate) struct MatcherFilterer {
    matcher: Matcher,
}

impl MatcherFilterer {
    pub(crate) fn new(matcher: Matcher) -> Self {
        Self { matcher }
    }
}

impl Filterable<types::Entry> for MatcherFilterer {
    fn filter(&self, entry: &types::Entry) -> super::FilterResult {
        if self
            .matcher
            .r#match(entry.title.as_deref().unwrap_or_default())
            || self
                .matcher
                .r#match(entry.feed_title.as_deref().unwrap_or_default())
        {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }
}

impl Filterable<types::Feed> for MatcherFilterer {
    fn filter(&self, feed: &types::Feed) -> super::FilterResult {
        if self
            .matcher
            .r#match(feed.title.as_deref().unwrap_or_default())
            || self
                .matcher
                .r#match(feed.website_url.as_deref().unwrap_or_default())
        {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }
}

impl Filterable<Notification> for MatcherFilterer {
    fn filter(&self, n: &Notification) -> super::FilterResult {
        if self.matcher.r#match(n.title())
            || self.matcher.r#match(&n.repository.owner)
            || self.matcher.r#match(&n.repository.name)
        {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }
}
