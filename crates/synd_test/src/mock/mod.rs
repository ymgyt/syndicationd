use std::{collections::HashMap, ops::Add, time::Duration};

use axum::{
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Form, Json, Router,
};
use chrono::Utc;
use headers::{authorization::Bearer, Authorization, Header};
use synd_auth::{
    device_flow::{
        provider::google::DeviceAccessTokenRequest as GoogleDeviceAccessTokenRequest,
        DeviceAccessTokenRequest, DeviceAccessTokenResponse, DeviceAuthorizationRequest,
        DeviceAuthorizationResponse,
    },
    jwt,
};
use tokio::net::TcpListener;

use crate::{certificate_buff, private_key_buff, TEST_EMAIL};

mod feed;

const DUMMY_GOOGLE_JWT_KEY_ID: &str = "dummy-google-jwt-kid-1";

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
        interval: None,
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
        interval: None,
    };

    Ok(Json(res))
}

async fn github_device_access_token(
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
    let jwt = {
        let header = jsonwebtoken::Header {
            typ: Some("JST".into()),
            // google use Allgorithm::RS256, but our testing private key use ECDSA
            alg: jsonwebtoken::Algorithm::ES256,
            kid: Some(DUMMY_GOOGLE_JWT_KEY_ID.to_owned()),
            ..Default::default()
        };
        let encoding_key =
            jsonwebtoken::EncodingKey::from_ec_pem(private_key_buff().as_slice()).unwrap();
        let claims = jwt::google::Claims {
            iss: "https://accounts.google.com".into(),
            azp: "dummy_google_client_id".into(),
            aud: "dummy_google_client_id".into(),
            sub: "123456789".into(),
            email: "ymgyt@ymgyt.io".into(),
            email_verified: true,
            iat: Utc::now().timestamp(),
            exp: Utc::now().add(Duration::from_secs(60 * 60)).timestamp(),
        };

        jsonwebtoken::encode(&header, &claims, &encoding_key).unwrap()
    };

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

    let response = serde_json::json!({
        "data": {
            "viewer": {
                "email": TEST_EMAIL,
            }
        }
    });
    Ok(Json(response))
}

// mock https://www.googleapis.com/oauth2/v1/certs
async fn google_jwt_pem() -> Json<HashMap<String, String>> {
    let key_id = DUMMY_GOOGLE_JWT_KEY_ID.to_owned();
    let cert = certificate_buff();
    Json([(key_id, cert)].into_iter().collect())
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
        .route("/feed/:feed", get(feed::feed));

    axum::serve(listener, router).await?;

    Ok(())
}
