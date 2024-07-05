use serde::Deserialize;
use serde_json::json;

pub mod notifications {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use axum::{
        extract::Query,
        response::{IntoResponse, Response},
    };
    use chrono::{TimeZone, Utc};

    // /notifications?all=false&participating=false&per_page=40&page=1 otel.name="HTTP" otel.kind="client"}: octocrab: /Users/ymgyt/.cargo/registry/src/index.crates.io-6f1
    #[allow(unused)]
    #[derive(Deserialize, Debug)]
    pub struct Notifications {
        all: bool,
        participating: bool,
        per_page: u8,
        page: u8,
    }

    pub async fn list(Query(n): Query<Notifications>) -> Response {
        tracing::debug!("{n:?}");

        let updated_at = Utc::with_ymd_and_hms(&Utc, 2024, 5, 4, 8, 0, 0).unwrap();

        if n.page == 1 {
            let notifications = json!({
                "items": [
                  {
                      "id": 1,
                      "repository": {
                          "id": 1,
                          "name": "repo-1",
                          "full_name": "org-1/repo-1",
                          "private:": false,
                          "url": "https://github.ymgyt.io/org-1/repo-1/",
                      },
                      "subject": {
                          "title": "title AA1",
                          "url": "https://test.ymgyt.io/subject/not/yet",
                          "type": "pullrequest",
                      },
                      "reason": "notyet",
                      "unread": true,
                      "updated_at": updated_at,
                      "url": "https://test.ymgyt.io/notifications/100",
                  },
                  {
                      "id": 2,
                      "repository": {
                          "id": 1,
                          "name": "repo-1",
                          "full_name": "org-1/repo-1",
                          "private:": false,
                          "url": "https://github.ymgyt.io/org-1/repo-1/",
                      },
                      "subject": {
                          "title": "title AA2",
                          "url": "https://test.ymgyt.io/subject/not/yet",
                          "type": "pullrequest",
                      },
                      "reason": "notyet",
                      "unread": true,
                      "updated_at": updated_at,
                      "url": "https://test.ymgyt.io/notifications/100",
                  }
                ],
            });
            tracing::debug!("{}", notifications.to_string());

            notifications.to_string().into_response()
        } else {
            json!({
                "items": [],
            })
            .to_string()
            .into_response()
        }
    }
}
