use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::{DateTime, TimeZone, Utc};
use jsonwebtoken::{DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{config, USER_AGENT};

type Kid = String;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("fetch pem: {0}")]
    FetchPem(#[from] reqwest::Error),
    #[error("decoding key pem not found")]
    DecodingKeyPemNotFound,
    #[error("decode id token: {0}")]
    Decode(#[from] jsonwebtoken::errors::Error),
    #[error("invalid jwt header: {0}")]
    InvalidHeader(String),
    #[error("refresh id token: {0}")]
    RefreshToken(reqwest::Error),
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Claims {
    iss: String,
    azp: String,
    aud: String,
    sub: String,
    pub email: String,
    pub email_verified: bool,
    iat: i64,
    exp: i64,
}

impl Claims {
    /// Return this `Claims` expire at given `DateTime`
    pub fn is_expired_at(&self, t: DateTime<Utc>) -> bool {
        self.exp < t.timestamp()
    }

    /// Return `DateTime` at when `Claims` expire
    pub fn expired_at(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(self.exp, 0)
            .single()
            .unwrap_or_else(Utc::now)
    }
}

#[derive(Clone)]
pub struct JwtService {
    client: Client,
    client_id: Cow<'static, str>,
    client_secret: Cow<'static, str>,
    key_cache: Arc<RwLock<HashMap<Kid, Arc<DecodingKey>>>>,
}

impl Default for JwtService {
    fn default() -> Self {
        Self::new(config::google::CLIENT_ID, config::google::CLIENT_ID2)
    }
}

impl JwtService {
    const PEM_ENDPOINT: &'static str = "https://www.googleapis.com/oauth2/v1/certs";
    const TOKEN_ENDPOINT: &'static str = "https://oauth2.googleapis.com/token";
    const ISSUERS: &'static [&'static str] =
        &["https://accounts.google.com", "accounts.google.com"];

    pub fn new(
        client_id: impl Into<Cow<'static, str>>,
        client_secret: impl Into<Cow<'static, str>>,
    ) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        Self {
            client,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            key_cache: Arc::new(RwLock::default()),
        }
    }

    /// Decode and validate JWT id token
    pub async fn decode_id_token(&self, id_token: &str) -> Result<Claims, JwtError> {
        // decode header to get kid
        let header = jsonwebtoken::decode_header(id_token).map_err(JwtError::Decode)?;
        let kid = header
            .kid
            .ok_or_else(|| JwtError::InvalidHeader("kid not found".into()))?;
        let decoding_key = self.lookup_decoding_pem(&kid).await?;
        let validation = {
            let mut v = Validation::new(header.alg);
            v.set_audience(&[self.client_id.as_ref()]);
            v.set_issuer(Self::ISSUERS);
            v.set_required_spec_claims(&["exp"]);
            v.validate_exp = true;
            v
        };

        jsonwebtoken::decode(id_token, &decoding_key, &validation)
            .map_err(JwtError::Decode)
            .map(|data| data.claims)
    }

    /// Decode JWT id token without signature validation
    pub fn decode_id_token_insecure(
        &self,
        id_token: &str,
        validate_exp: bool,
    ) -> Result<Claims, JwtError> {
        let decoding_key = DecodingKey::from_secret(&[]);
        let validation = {
            let mut v = Validation::default();
            v.insecure_disable_signature_validation();
            v.set_audience(&[self.client_id.as_ref()]);
            v.set_issuer(Self::ISSUERS);
            v.set_required_spec_claims(&["exp"]);
            v.validate_exp = validate_exp;
            v
        };

        jsonwebtoken::decode(id_token, &decoding_key, &validation)
            .map_err(JwtError::Decode)
            .map(|data| data.claims)
    }

    async fn lookup_decoding_pem(&self, kid: &str) -> Result<Arc<DecodingKey>, JwtError> {
        if let Some(key) = self.key_cache.read().unwrap().get(kid) {
            return Ok(key.clone());
        }

        self.refresh_key_cache().await?;

        self.key_cache
            .read()
            .unwrap()
            .get(kid)
            .cloned()
            .ok_or(JwtError::DecodingKeyPemNotFound)
    }

    async fn refresh_key_cache(&self) -> Result<(), JwtError> {
        let keys = self
            .fetch_decoding_key_pem()
            .await?
            .into_iter()
            .filter_map(|key| match DecodingKey::from_rsa_pem(key.pem.as_bytes()) {
                Ok(de_key) => Some((key.kid, Arc::new(de_key))),
                Err(err) => {
                    tracing::warn!("failed to create jwt decoding key from pem: {err}");
                    None
                }
            });

        let mut cache = self.key_cache.write().unwrap();
        keys.for_each(|(kid, de_key)| {
            cache.insert(kid, de_key);
        });

        Ok(())
    }

    async fn fetch_decoding_key_pem(&self) -> Result<Vec<DecodingKeyPem>, JwtError> {
        async fn call(
            client: &Client,
            endpoint: &str,
        ) -> Result<HashMap<String, String>, reqwest::Error> {
            let payload = client
                .get(endpoint)
                .header(http::header::ACCEPT, "application/json")
                .send()
                .await?
                .error_for_status()?
                .json::<HashMap<String, String>>()
                .await?;
            Ok(payload)
        }

        let payload = call(&self.client, Self::PEM_ENDPOINT)
            .await
            .map_err(JwtError::FetchPem)?;

        Ok(payload
            .into_iter()
            .map(|(kid, pem)| DecodingKeyPem { kid, pem })
            .collect())
    }

    /// Refresh id token
    /// <https://developers.google.com/identity/gsi/web/guides/devices#obtain_an_id_token_and_refresh_token>
    pub async fn refresh_id_token(&self, refresh_token: &str) -> Result<String, JwtError> {
        #[derive(Serialize)]
        struct Request<'s> {
            client_id: &'s str,
            client_secret: &'s str,
            refresh_token: &'s str,
            grant_type: &'static str,
        }

        #[derive(Deserialize)]
        struct Response {
            #[allow(dead_code)]
            expires_in: i64,
            id_token: String,
        }

        async fn call<'s>(
            client: &Client,
            endpoint: &str,
            payload: &Request<'s>,
        ) -> Result<Response, reqwest::Error> {
            client
                .post(endpoint)
                .header(http::header::ACCEPT, "application/json")
                .form(payload)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
        }

        // https://developers.google.com/identity/protocols/oauth2/limited-input-device#offline
        let request = &Request {
            client_id: self.client_id.as_ref(),
            client_secret: self.client_secret.as_ref(),
            refresh_token,
            grant_type: "refresh_token",
        };
        let response = call(&self.client, Self::TOKEN_ENDPOINT, request)
            .await
            .map_err(JwtError::RefreshToken)?;

        Ok(response.id_token)
    }
}

struct DecodingKeyPem {
    kid: String,
    pem: String,
}
