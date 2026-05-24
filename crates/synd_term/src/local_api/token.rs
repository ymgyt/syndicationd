use rand::distr::{Alphanumeric, SampleString};

#[derive(Clone, Debug)]
pub(super) struct LocalApiToken(String);

impl LocalApiToken {
    pub(super) fn generate() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::rng(), 64))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn into_string(self) -> String {
        self.0
    }
}
