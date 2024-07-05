use std::{collections::HashMap, sync::atomic::AtomicUsize, time::Duration};

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Form, Json, Router,
};
use headers::{authorization::Bearer, Authorization, Header};
use serde::Serialize;
use synd_auth::device_flow::{
    provider::google::DeviceAccessTokenRequest as GoogleDeviceAccessTokenRequest,
    DeviceAccessTokenErrorResponse, DeviceAccessTokenRequest, DeviceAccessTokenResponse,
    DeviceAuthorizationRequest, DeviceAuthorizationResponse,
};
use tokio::net::TcpListener;

use crate::{certificate_buff, jwt::DUMMY_GOOGLE_JWT_KEY_ID, GITHUB_INVALID_TOKEN, TEST_EMAIL};

mod feed;

async fn github_device_authorization(
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
        interval: Some(1), // for test speed
    };

    Ok(Json(res))
}

async fn google_device_authorization(
    Form(DeviceAuthorizationRequest { scope, .. }): Form<DeviceAuthorizationRequest<'static>>,
) -> Result<Json<DeviceAuthorizationResponse>, StatusCode> {
    tracing::debug!(%scope, "Handle device authorization request");

    if scope != "email" {
        return Err(StatusCode::BAD_REQUEST);
    }
    let res = DeviceAuthorizationResponse {
        device_code: "DCGGL1".into(),
        user_code: "UCGGL1".into(),
        verification_uri: Some("https://syndicationd.ymgyt.io/test".parse().unwrap()),
        verification_url: None,
        verification_uri_complete: None,
        expires_in: 3600,
        interval: Some(1), // for test speed
    };

    Ok(Json(res))
}

async fn github_device_access_token(
    Form(DeviceAccessTokenRequest { device_code, .. }): Form<DeviceAccessTokenRequest<'static>>,
) -> Response {
    // Check error handling
    static TRY: AtomicUsize = AtomicUsize::new(0);
    let count = TRY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    tracing::debug!("Handle device access token request");

    if device_code != "DC001" {
        return StatusCode::BAD_REQUEST.into_response();
    }

    match count {
        0 => (
            StatusCode::BAD_REQUEST,
            Json(DeviceAccessTokenErrorResponse {
                error: synd_auth::device_flow::DeviceAccessTokenErrorCode::AuthorizationPending,
                error_description: None,
                error_uri: None,
            }),
        )
            .into_response(),
        1 => (
            StatusCode::PRECONDITION_REQUIRED,
            Json(DeviceAccessTokenErrorResponse {
                error: synd_auth::device_flow::DeviceAccessTokenErrorCode::SlowDown,
                error_description: None,
                error_uri: None,
            }),
        )
            .into_response(),
        _ => {
            let res = DeviceAccessTokenResponse {
                access_token: "gh_dummy_access_token".into(),
                token_type: String::new(),
                expires_in: None,
                refresh_token: None,
                id_token: None,
            };

            Json(res).into_response()
        }
    }
}

async fn google_device_access_token(
    Form(GoogleDeviceAccessTokenRequest { code, .. }): Form<
        GoogleDeviceAccessTokenRequest<'static>,
    >,
) -> Result<Json<DeviceAccessTokenResponse>, StatusCode> {
    tracing::debug!("Handle device access token request");

    if code != "DCGGL1" {
        return Err(StatusCode::BAD_REQUEST);
    }
    // mock user input duration
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Generate jwt
    let jwt = crate::jwt::google_test_jwt();

    let res = DeviceAccessTokenResponse {
        access_token: "gh_dummy_access_token".into(),
        token_type: String::new(),
        expires_in: Some(600),
        refresh_token: Some("dummy_refresh_token".into()),
        id_token: Some(jwt),
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

    if token == GITHUB_INVALID_TOKEN {
        Err(StatusCode::UNAUTHORIZED)
    } else {
        let response = serde_json::json!({
            "data": {
                "viewer": {
                    "email": TEST_EMAIL,
                }
            }
        });
        Ok(Json(response))
    }
}

// mock https://www.googleapis.com/oauth2/v1/certs
async fn google_jwt_pem() -> Json<HashMap<String, String>> {
    let key_id = DUMMY_GOOGLE_JWT_KEY_ID.to_owned();
    let cert = certificate_buff();
    Json([(key_id, cert)].into_iter().collect())
}

#[derive(Serialize)]
struct GoogleOauth2TokenResponse {
    expires_in: i64,
    id_token: String,
}
// mock https://oauth2.googleapis.com/token
async fn google_oauth2_token() -> Json<GoogleOauth2TokenResponse> {
    let id_token = crate::jwt::google_test_jwt();
    let expires_in = 60 * 30;
    Json(GoogleOauth2TokenResponse {
        expires_in,
        id_token,
    })
}

pub async fn serve(listener: TcpListener) -> anyhow::Result<()> {
    let case_1 = Router::new()
        .route(
            "/github/login/device/code",
            post(github_device_authorization),
        )
        .route(
            "/github/login/oauth/access_token",
            post(github_device_access_token),
        )
        .route(
            "/google/login/device/code",
            post(google_device_authorization),
        )
        .route(
            "/google/login/oauth/access_token",
            post(google_device_access_token),
        );
    let router = Router::new()
        .nest("/case1", case_1)
        .route("/github/graphql", post(github_graphql_viewer))
        .route("/google/oauth2/v1/certs", get(google_jwt_pem))
        .route("/google/oauth2/token", post(google_oauth2_token))
        .route("/feed/error/:error", get(feed::feed_error))
        .route("/feed/:feed", get(feed::feed))
        .layer(axum::middleware::from_fn(debug_mw));

    let addr = listener.local_addr().ok();
    tracing::info!(?addr, "Serving...");
    axum::serve(listener, router).await?;

    Ok(())
}

async fn debug_mw(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    tracing::debug!("req:?");
    next.run(req).await
}
