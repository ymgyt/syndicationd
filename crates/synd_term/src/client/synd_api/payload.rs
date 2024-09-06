use crate::{client::synd_api::query, types};

#[derive(Debug, Clone)]
pub struct FetchEntriesPayload {
    pub entries: Vec<types::Entry>,
    pub page_info: types::PageInfo,
}

impl From<query::entries::EntriesOutput> for FetchEntriesPayload {
    fn from(v: query::entries::EntriesOutput) -> Self {
        let page_info = v.entries.page_info.into();
        let entries = v.entries.nodes.into_iter().map(Into::into).collect();

        Self { entries, page_info }
    }
}

pub struct ExportSubscriptionPayload {
    pub feeds: Vec<types::ExportedFeed>,
    pub page_info: types::PageInfo,
}

impl From<query::export_subscription::ExportSubscriptionOutput> for ExportSubscriptionPayload {
    fn from(v: query::export_subscription::ExportSubscriptionOutput) -> Self {
        Self {
            feeds: v.feeds.nodes.into_iter().map(Into::into).collect(),
            page_info: v.feeds.page_info.into(),
        }
    }
}
