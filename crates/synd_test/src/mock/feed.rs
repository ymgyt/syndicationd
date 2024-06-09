use axum::{extract::Path, response::IntoResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct FeedParams {
    feed: String,
}

pub(super) async fn feed(Path(FeedParams { feed }): Path<FeedParams>) -> impl IntoResponse {
    let content = match feed.as_str() {
        "twir_atom" => include_str!("feeddata/twir_atom.xml"),
        "o11y_news" => include_str!("feeddata/o11y_news_rss.xml"),
        _ => unreachable!("undefined feed fixture posted"),
    };

    content.into_response()
}
