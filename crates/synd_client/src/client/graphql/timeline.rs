use tracing::instrument;

use super::GraphqlRequest;
use crate::{
    Client, SyndApiError,
    payload::{
        CreateTimelineInput, CreateTimelinePayload, Timeline, TimelineChangesPayload,
        TimelineEntriesRequest, TimelineEntryConnection, TimelineId,
    },
};

const TIMELINE_CHANGES_QUERY: &str = include_str!("query/timeline_changes.gql");
const TIMELINE_ENTRIES_QUERY: &str = concat!(
    include_str!("query/timeline_entries.gql"),
    include_str!("query/timeline_entries_page.gql"),
);
const TIMELINE_ENTRIES_IN_RANGE_QUERY: &str = concat!(
    include_str!("query/timeline_entries_in_range.gql"),
    include_str!("query/timeline_entries_page.gql"),
);
const LIST_TIMELINES_QUERY: &str = include_str!("query/list_timelines.gql");
const CREATE_TIMELINE_MUTATION: &str = include_str!("query/create_timeline.gql");

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListTimelinesData {
    feed_registry: Timelines,
}

#[derive(Debug, serde::Deserialize)]
struct Timelines {
    timelines: Vec<Timeline>,
}

#[derive(Debug, serde::Serialize)]
struct CreateTimelineVariables {
    input: CreateTimelineInput,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTimelineData {
    create_timeline: CreateTimelinePayload,
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
#[serde(rename_all = "camelCase")]
struct TimelineChangesVariables {
    timeline_id: TimelineId,
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
    pub async fn list_timelines(&self) -> Result<Vec<Timeline>, SyndApiError> {
        let data: ListTimelinesData = self
            .execute_graphql(&GraphqlRequest::new(
                LIST_TIMELINES_QUERY,
                serde_json::json!({}),
            ))
            .await?
            .require_complete()?;
        Ok(data.feed_registry.timelines)
    }

    #[instrument(skip(self))]
    pub async fn create_timeline(
        &self,
        input: CreateTimelineInput,
    ) -> Result<CreateTimelinePayload, SyndApiError> {
        let data: CreateTimelineData = self
            .execute_graphql(&GraphqlRequest::new(
                CREATE_TIMELINE_MUTATION,
                CreateTimelineVariables { input },
            ))
            .await?
            .require_complete()?;
        Ok(data.create_timeline)
    }

    #[instrument(skip(self))]
    pub async fn fetch_timeline_entries(
        &self,
        request: TimelineEntriesRequest,
    ) -> Result<TimelineEntryConnection, SyndApiError> {
        if let Some(width @ 1001..) = request.max_unicode_width() {
            return Err(SyndApiError::InvalidExcerptWidth(width));
        }
        // Older daemons reject unknown field arguments even when their values are null.
        let query = if request.has_time_range() {
            TIMELINE_ENTRIES_IN_RANGE_QUERY
        } else {
            TIMELINE_ENTRIES_QUERY
        };
        let outcome = self
            .execute_graphql::<_, TimelineEntriesData>(&GraphqlRequest::new(query, request))
            .await?
            .accept_partial()?;
        outcome.warn_partial_errors();
        Ok(outcome.into_data().into())
    }

    #[instrument(skip(self))]
    pub async fn fetch_timeline_changes(
        &self,
        timeline_id: TimelineId,
        since: i64,
        first: i64,
    ) -> Result<TimelineChangesPayload, SyndApiError> {
        let outcome = self
            .execute_graphql::<_, TimelineChangesData>(&GraphqlRequest::new(
                TIMELINE_CHANGES_QUERY,
                TimelineChangesVariables {
                    timeline_id,
                    since,
                    first,
                },
            ))
            .await?
            .accept_partial()?;
        outcome.warn_partial_errors();
        Ok(outcome.into_data().into())
    }
}
