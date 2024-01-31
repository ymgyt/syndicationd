use axum::{http::StatusCode, routing::post, Form, Json, Router};
use synd_term::auth::device_flow::{DeviceAuthorizationRequest, DeviceAuthorizationResponse};
use tokio::net::TcpListener;

async fn device_authorization(
    Form(DeviceAuthorizationRequest { scope, .. }): Form<DeviceAuthorizationRequest<'static>>,
) -> Result<Json<DeviceAuthorizationResponse>, StatusCode> {
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

pub async fn serve(listener: TcpListener) -> anyhow::Result<()> {
    let router = Router::new().route("/github/login/device/code", post(device_authorization));

    axum::serve(listener, router).await?;

    Ok(())
}
