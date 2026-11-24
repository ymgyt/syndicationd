use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_client::payload::PageInfo;

use crate::error::ToolError;

#[derive(Deserialize, JsonSchema)]
pub(super) struct PageParams {
    /// Cursor returned as `next_cursor` by the previous page of this tool.
    after: Option<String>,
    /// Maximum number of items to return. Defaults to 20.
    #[schemars(range(min = 1, max = 100))]
    first: Option<i64>,
}

impl PageParams {
    pub(super) fn first(&self) -> Result<i64, ToolError> {
        match self.first.unwrap_or(20) {
            first @ 1..=100 => Ok(first),
            _ => Err(ToolError::InvalidPageSize),
        }
    }

    pub(super) fn into_after(self) -> Option<String> {
        self.after
    }
}

#[derive(Serialize, JsonSchema)]
pub(super) struct Page<T> {
    items: Vec<T>,
    /// Pass this as `after` to fetch the next page. Null on the last page.
    next_cursor: Option<String>,
}

impl<T> Page<T> {
    pub(super) fn from_nodes<N>(nodes: Vec<N>, page_info: PageInfo) -> Self
    where
        T: From<N>,
    {
        let next_cursor = match page_info {
            PageInfo::Complete { .. } => None,
            PageInfo::More { next_cursor } => Some(next_cursor),
        };
        Self {
            items: nodes.into_iter().map(T::from).collect(),
            next_cursor,
        }
    }
}
