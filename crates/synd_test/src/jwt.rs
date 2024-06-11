use std::{
    ops::{Add, Sub},
    time::Duration,
};

use chrono::{DateTime, Utc};
use synd_auth::jwt;

use crate::{private_key_buff, TEST_EMAIL};

pub(super) const DUMMY_GOOGLE_JWT_KEY_ID: &str = "dummy-google-jwt-kid-1";
pub const DUMMY_GOOGLE_CLIENT_ID: &str = "dummy_google_client_id";

pub(super) fn google_test_jwt() -> String {
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
        azp: DUMMY_GOOGLE_CLIENT_ID.to_owned(),
        aud: DUMMY_GOOGLE_CLIENT_ID.to_owned(),
        sub: "123456789".into(),
        email: TEST_EMAIL.to_owned(),
        email_verified: true,
        iat: Utc::now().timestamp(),
        exp: Utc::now().add(Duration::from_secs(60 * 60)).timestamp(),
    };

    jsonwebtoken::encode(&header, &claims, &encoding_key).unwrap()
}

/// Return encoded expired jwt and `expired_at`
pub fn google_expired_jwt() -> (String, DateTime<Utc>) {
    let header = jsonwebtoken::Header {
        typ: Some("JST".into()),
        // google use Allgorithm::RS256, but our testing private key use ECDSA
        alg: jsonwebtoken::Algorithm::ES256,
        kid: Some(DUMMY_GOOGLE_JWT_KEY_ID.to_owned()),
        ..Default::default()
    };
    let encoding_key =
        jsonwebtoken::EncodingKey::from_ec_pem(private_key_buff().as_slice()).unwrap();
    let iat = Utc::now().sub(Duration::from_secs(60 * 60 * 24));
    let exp = Utc::now()
        .sub(Duration::from_secs(60 * 60 * 24))
        .add(Duration::from_secs(60 * 60));
    let claims = jwt::google::Claims {
        iss: "https://accounts.google.com".into(),
        azp: DUMMY_GOOGLE_CLIENT_ID.to_owned(),
        aud: DUMMY_GOOGLE_CLIENT_ID.to_owned(),
        sub: "123456789".into(),
        email: TEST_EMAIL.to_owned(),
        email_verified: true,
        iat: iat.timestamp(),
        exp: exp.timestamp(),
    };

    (
        jsonwebtoken::encode(&header, &claims, &encoding_key).unwrap(),
        exp,
    )
}
