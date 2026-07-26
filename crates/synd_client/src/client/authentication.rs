use reqwest::header::{self, HeaderMap, HeaderValue};

use crate::SyndApiError;

pub enum ApiCredential {
    Github { access_token: String },
    Google { id_token: String },
    LocalBearer { token: String },
}

impl TryFrom<ApiCredential> for HeaderValue {
    type Error = header::InvalidHeaderValue;

    fn try_from(credential: ApiCredential) -> Result<Self, Self::Error> {
        let value = match credential {
            ApiCredential::Github { access_token } => format!("github {access_token}"),
            ApiCredential::Google { id_token } => format!("google {id_token}"),
            ApiCredential::LocalBearer { token } => format!("Bearer {token}"),
        };
        let mut value = HeaderValue::try_from(value)?;
        value.set_sensitive(true);
        Ok(value)
    }
}

#[derive(Clone)]
pub(super) enum ClientAuthentication {
    Required,
    Header(HeaderValue),
    TransportTrusted,
}

impl ClientAuthentication {
    pub(super) fn configure(&mut self, credential: ApiCredential) -> Result<(), SyndApiError> {
        *self = Self::Header(credential.try_into()?);
        Ok(())
    }

    pub(super) fn apply_authorization_header(
        &self,
        headers: &mut HeaderMap,
    ) -> Result<(), SyndApiError> {
        match self {
            Self::Required => Err(SyndApiError::MissingCredential),
            Self::Header(value) => {
                headers.insert(header::AUTHORIZATION, value.clone());
                Ok(())
            }
            Self::TransportTrusted => Ok(()),
        }
    }
}
