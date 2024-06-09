use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct FeedParams {
    feed: String,
}

pub(super) async fn feed(Path(FeedParams { feed }): Path<FeedParams>) -> impl IntoResponse {
    let content = match feed.as_str() {
        "twir_atom" => include_str!("feeddata/twir_atom.xml"),
        "o11y_news" => include_str!("feeddata/o11y_news_rss.xml"),
        x => {
            tracing::warn!("feed {x} undefined");
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    content.into_response()
}
