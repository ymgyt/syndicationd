use serde::Deserialize;
use serde_json::json;

pub mod notifications {
    use std::{ops::Sub, sync::LazyLock};

    #[allow(clippy::wildcard_imports)]
    use super::*;
    use axum::{
        Json,
        extract::{Path, Query},
        http::StatusCode,
        response::{IntoResponse, Response},
    };
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use serde::Serialize;
    use serde_json::Value;

    #[allow(unused)]
    #[derive(Deserialize, Debug)]
    pub struct Notifications {
        all: bool,
        participating: bool,
        per_page: u8,
        page: u8,
    }

    pub static NOW: LazyLock<DateTime<Utc>> =
        LazyLock::new(|| Utc::with_ymd_and_hms(&Utc, 2024, 7, 5, 8, 0, 0).unwrap());

    pub async fn list(Query(n): Query<Notifications>) -> Response {
        if n.page == 1 {
            let notifications = json!({
                "items": [
                  {
                      "id": 1,
                      "repository": repo_a(),
                      "subject": {
                          "title": "title AA1",
                          "url": "https://api.ymgyt.io/repos/sakura/repo-1/issues/1",
                          "type": "issue",
                      },
                      "reason": "mention",
                      "unread": true,
                      "updated_at": NOW.sub(Duration::hours(1)),
                      "url": "https://api.ymgyt.io/notifications/threads/1",
                  },
                  {
                      "id": 2,
                      "repository": repo_a(),
                      "subject": {
                          "title": "title AA2",
                          "url": "https://api.ymgyt.io/repos/sakura/repo-a/pulls/1",
                          "type": "pullrequest",
                      },
                      "reason": "mention",
                      "unread": true,
                      "updated_at": NOW.sub(Duration::hours(2)),
                      "url": "https://api.ymgyt.io/notifications/threads/2",
                  },
                  {
                      "id": 3,
                      "repository": repo_private(),
                      "subject": {
                          "title": "Add feature foo",
                          "url": "https://api.ymgyt.io/repos/sakura/repo-private/pulls/2",
                          "type": "pullrequest",
                      },
                      "reason": "subscribed",
                      "unread": true,
                      "updated_at": NOW.sub(Duration::hours(3)),
                      "url": "https://api.ymgyt.io/notifications/threads/3",
                  },
                  {
                      "id": 4,
                      "repository": repo_private(),
                      "subject": {
                          "title": "Request Review",
                          "url": "https://api.ymgyt.io/repos/sakura/repo-private/pulls/3",
                          "type": "pullrequest",
                      },
                      "reason": "review_requested",
                      "unread": true,
                      "updated_at": NOW.sub(Duration::hours(4)),
                      "url": "https://api.ymgyt.io/notifications/threads/4",
                  }
                ],
            });

            notifications.to_string().into_response()
        } else {
            json!({
                "items": [],
            })
            .to_string()
            .into_response()
        }
    }

    fn repo_a() -> Value {
        json!({
          "id": 1,
          "name": "repo-a",
          "full_name": "sakura/repo-a",
          "private": false,
          "url": "https://github.ymgyt.io/sakura/repo-a/",
        })
    }

    fn repo_private() -> Value {
        json!({
          "id": 2,
          "name": "repo-private",
          "full_name": "sakura/repo-private",
          "private": true,
          "url": "https://github.ymgyt.io/sakura/repo-private/",
        })
    }

    #[derive(Deserialize)]
    pub struct MarkAsDoneParams {
        thread: u64,
    }

    pub async fn mark_as_done(
        Path(MarkAsDoneParams { thread }): Path<MarkAsDoneParams>,
    ) -> impl IntoResponse {
        tracing::info!("Mark as done thread: {thread}");
        StatusCode::OK
    }

    #[derive(Deserialize)]
    pub struct UnsubscribeThreadParams {
        thread: u64,
    }
    #[derive(Serialize)]
    pub struct UnsubscribeThreadResponse {}
    pub async fn unsubscribe_thread(
        Path(UnsubscribeThreadParams { thread }): Path<UnsubscribeThreadParams>,
    ) -> Json<UnsubscribeThreadResponse> {
        tracing::info!("Unsubscribe thread: {thread}");
        Json(UnsubscribeThreadResponse {})
    }
}

pub mod gql {

    use axum::{
        Json,
        response::{IntoResponse, Response},
    };
    use serde::Deserialize;
    use serde_json::Value;

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Payload {
        operation_name: String,
        _query: String,
        variables: Value,
    }

    pub async fn graphql(Json(payload): Json<Payload>) -> Response {
        match payload.operation_name.as_str() {
            "IssueQuery" => {
                let issue = payload.variables["issueNumber"].as_u64().unwrap();
                let repo_owner = payload.variables["repositoryOwner"].as_str().unwrap();
                let repo_name = payload.variables["repositoryName"].as_str().unwrap();
                let fixture = format!("{repo_owner}_{repo_name}_issues_{issue}");
                match fixture.as_str() {
                    "sakura_repo-a_issues_1" => {
                        include_str!("./githubdata/sakura_repo-a_issues_1.json").into_response()
                    }
                    _ => panic!("Unexpected issue fixture: {fixture}"),
                }
            }
            "PullRequestQuery" => {
                let pr = payload.variables["pullRequestNumber"].as_u64().unwrap();
                let repo_owner = payload.variables["repositoryOwner"].as_str().unwrap();
                let repo_name = payload.variables["repositoryName"].as_str().unwrap();
                let fixture = format!("{repo_owner}_{repo_name}_prs_{pr}");
                match fixture.as_str() {
                    "sakura_repo-a_prs_1" => {
                        include_str!("./githubdata/sakura_repo-a_prs_1.json").into_response()
                    }
                    "sakura_repo-private_prs_2" => {
                        include_str!("./githubdata/sakura_repo-private_prs_2.json").into_response()
                    }
                    "sakura_repo-private_prs_3" => {
                        include_str!("./githubdata/sakura_repo-private_prs_3.json").into_response()
                    }
                    _ => panic!("Unexpected pr fixture: {fixture}"),
                }
            }
            unexpected => panic!("Unexpected operation: {unexpected}"),
        }
    }
}
