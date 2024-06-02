use std::time::Duration;

use axum::{
    http::{HeaderMap, StatusCode},
    routing::post,
    Form, Json, Router,
};
use headers::{authorization::Bearer, Authorization, Header};
use synd_auth::device_flow::{
    DeviceAccessTokenRequest, DeviceAccessTokenResponse, DeviceAuthorizationRequest,
    DeviceAuthorizationResponse,
};
use tokio::net::TcpListener;

async fn device_authorization(
    Form(DeviceAuthorizationRequest { scope, .. }): Form<DeviceAuthorizationRequest<'static>>,
) -> Result<Json<DeviceAuthorizationResponse>, StatusCode> {
    tracing::debug!(%scope, "Handle device authorization request");

    if scope != "user:email" {
        return Err(StatusCode::BAD_REQUEST);
    }
    let res = DeviceAuthorizationResponse {
        device_code: "DC001".into(),
        user_code: "UC123456".into(),
        verification_uri: Some("https://syndicationd.ymgyt.io/test".parse().unwrap()),
        verification_url: None,
        verification_uri_complete: None,
        expires_in: 3600,
        interval: None,
    };

    Ok(Json(res))
}

async fn device_access_token(
    Form(DeviceAccessTokenRequest { device_code, .. }): Form<DeviceAccessTokenRequest<'static>>,
) -> Result<Json<DeviceAccessTokenResponse>, StatusCode> {
    tracing::debug!("Handle device access token request");

    if device_code != "DC001" {
        return Err(StatusCode::BAD_REQUEST);
    }
    // mock user input duration
    tokio::time::sleep(Duration::from_secs(1)).await;

    let res = DeviceAccessTokenResponse {
        access_token: "gh_dummy_access_token".into(),
        token_type: String::new(),
        expires_in: None,
        refresh_token: None,
        id_token: None,
    };

    Ok(Json(res))
}

async fn github_graphql_viewer(
    headers: HeaderMap,
    _query: String,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth = headers.get(Authorization::<Bearer>::name()).unwrap();
    let auth = Authorization::<Bearer>::decode(&mut std::iter::once(auth)).unwrap();
    let token = auth.token();

    tracing::debug!("Got token: `{token}`");

    let dummy_email = "ymgyt@ymgyt.io";

    let response = serde_json::json!({
        "data": {
            "viewer": {
                "email": dummy_email
            }
        }
    });
    Ok(Json(response))
}

pub async fn serve(listener: TcpListener) -> anyhow::Result<()> {
    let case_1 = Router::new()
        .route("/github/login/device/code", post(device_authorization))
        .route(
            "/github/login/oauth/access_token",
            post(device_access_token),
        );
    let router = Router::new()
        .nest("/case1", case_1)
        .route("/github/graphql", post(github_graphql_viewer));

    axum::serve(listener, router).await?;

    Ok(())
}
