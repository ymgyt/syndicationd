use chrono::{DateTime, Utc};

/// Database reference to one stored byte blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobRef {
    pk: i64,
}

impl BlobRef {
    pub fn new(pk: i64) -> Self {
        Self { pk }
    }

    pub fn pk(self) -> i64 {
        self.pk
    }
}

/// Command to store generic byte payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutBlobCommand {
    pub bytes: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

impl PutBlobCommand {
    pub fn new(bytes: Vec<u8>, created_at: DateTime<Utc>) -> Self {
        Self { bytes, created_at }
    }
}
