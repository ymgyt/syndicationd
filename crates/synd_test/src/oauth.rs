use std::time::Duration;

use axum::{http::StatusCode, routing::post, Form, Json, Router};
use synd_authn::device_flow::{
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
        verification_uri: "https://syndicationd.ymgyt.io/test".parse().unwrap(),
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
        access_token: "dummy".into(),
        token_type: String::new(),
        expires_in: None,
    };

    Ok(Json(res))
}

pub async fn serve(listener: TcpListener) -> anyhow::Result<()> {
    let case_1 = Router::new()
        .route("/github/login/device/code", post(device_authorization))
        .route(
            "/github/login/oauth/access_token",
            post(device_access_token),
        );
    let router = Router::new().nest("/case1", case_1);

    axum::serve(listener, router).await?;

    Ok(())
}
