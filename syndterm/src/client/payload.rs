use crate::{client::query, types};

#[derive(Debug)]
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
