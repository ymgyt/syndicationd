use crate::client::query;

#[derive(Debug)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

impl From<query::entries::PageInfo> for PageInfo {
    fn from(v: query::entries::PageInfo) -> Self {
        Self {
            has_next_page: v.has_next_page,
            end_cursor: v.end_cursor,
        }
    }
}

impl From<query::export_subscription::ExportSubscriptionOutputFeedsPageInfo> for PageInfo {
    fn from(v: query::export_subscription::ExportSubscriptionOutputFeedsPageInfo) -> Self {
        Self {
            has_next_page: v.has_next_page,
            end_cursor: v.end_cursor,
        }
    }
}
