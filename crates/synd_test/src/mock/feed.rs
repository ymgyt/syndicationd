use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
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

#[derive(Deserialize)]
pub(super) struct FeedErrorParams {
    error: String,
}

pub(super) async fn feed_error(Path(FeedErrorParams { error }): Path<FeedErrorParams>) -> Response {
    match error.as_str() {
        "internal" => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        "malformed" => "malformed xml".into_response(),
        _ => unreachable!("undefined feed fixture posted"),
    }
}
