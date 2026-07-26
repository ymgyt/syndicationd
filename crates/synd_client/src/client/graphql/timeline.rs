use tracing::instrument;

use super::GraphqlRequest;
use crate::{
    Client, SyndApiError,
    payload::{TimelineChangesPayload, TimelineEntryConnection},
};

const TIMELINE_CHANGES_QUERY: &str = include_str!("query/timeline_changes.gql");
const TIMELINE_ENTRIES_QUERY: &str = include_str!("query/timeline_entries.gql");

#[derive(Debug, serde::Serialize)]
struct TimelineEntriesVariables {
    after: Option<String>,
    first: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TimelineEntriesData {
    output: TimelineEntriesOutput,
}

#[derive(Debug, serde::Deserialize)]
struct TimelineEntriesOutput {
    timeline: TimelineEntries,
}

#[derive(Debug, serde::Deserialize)]
struct TimelineEntries {
    entries: TimelineEntryConnection,
}

impl From<TimelineEntriesData> for TimelineEntryConnection {
    fn from(data: TimelineEntriesData) -> Self {
        data.output.timeline.entries
    }
}

#[derive(Debug, serde::Serialize)]
struct TimelineChangesVariables {
    since: i64,
    first: i64,
}

#[derive(Debug, serde::Deserialize)]
struct TimelineChangesData {
    output: TimelineChangesOutput,
}

#[derive(Debug, serde::Deserialize)]
struct TimelineChangesOutput {
    timeline: TimelineChanges,
}

#[derive(Debug, serde::Deserialize)]
struct TimelineChanges {
    changes: TimelineChangesPayload,
}

impl From<TimelineChangesData> for TimelineChangesPayload {
    fn from(data: TimelineChangesData) -> Self {
        data.output.timeline.changes
    }
}

impl Client {
    #[instrument(skip(self))]
    pub async fn fetch_timeline_entries(
        &self,
        after: Option<String>,
        first: i64,
    ) -> Result<TimelineEntryConnection, SyndApiError> {
        let outcome = self
            .execute_graphql::<_, TimelineEntriesData>(&GraphqlRequest::new(
                TIMELINE_ENTRIES_QUERY,
                TimelineEntriesVariables { after, first },
            ))
            .await?
            .accept_partial()?;
        outcome.warn_partial_errors();
        Ok(outcome.into_data().into())
    }

    #[instrument(skip(self))]
    pub async fn fetch_timeline_changes(
        &self,
        since: i64,
        first: i64,
    ) -> Result<TimelineChangesPayload, SyndApiError> {
        let outcome = self
            .execute_graphql::<_, TimelineChangesData>(&GraphqlRequest::new(
                TIMELINE_CHANGES_QUERY,
                TimelineChangesVariables { since, first },
            ))
            .await?
            .accept_partial()?;
        outcome.warn_partial_errors();
        Ok(outcome.into_data().into())
    }
}
