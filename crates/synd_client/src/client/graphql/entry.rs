use serde::Deserialize;
use serde_json::json;
use synd_feed::entry::EntryId;
use tracing::instrument;

use super::GraphqlRequest;
use crate::{Client, SyndApiError, payload::EntryDetails};

const GET_ENTRY_QUERY: &str = include_str!("query/get_entry.gql");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetEntryData {
    feed_registry: EntryLookup,
}

#[derive(Debug, Deserialize)]
struct EntryLookup {
    entry: Option<EntryDetails>,
}

impl Client {
    #[instrument(skip(self))]
    pub async fn get_entry(&self, id: &EntryId) -> Result<Option<EntryDetails>, SyndApiError> {
        let data: GetEntryData = self
            .execute_graphql(&GraphqlRequest::new(GET_ENTRY_QUERY, json!({ "id": id })))
            .await?
            .require_complete()?;
        Ok(data.feed_registry.entry)
    }
}
