use sha2::{Digest, Sha256};
use sqlx::{Row, Sqlite, Transaction};
use synd_registry::{
    BlobStoreTx, RegistryDbError, RegistryDbResult,
    crawl::blob::{BlobRef, PutBlobCommand},
};

const DIGEST_ALGO_SHA256: &str = "sha256";
const COMPRESSION_ALGO_ZSTD: &str = "zstd";
const ZSTD_LEVEL: i32 = 3;

pub(super) struct BlobTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> BlobTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn put(&mut self, command: PutBlobCommand) -> RegistryDbResult<BlobRef> {
        let digest = Sha256::digest(&command.bytes);
        let uncompressed_len = to_i64(command.bytes.len(), "uncompressed blob length")?;
        let compressed = zstd::bulk::compress(command.bytes.as_slice(), ZSTD_LEVEL)
            .map_err(RegistryDbError::internal)?;
        let compressed_len = to_i64(compressed.len(), "compressed blob length")?;
        let compression_opts = serde_json::json!({ "level": ZSTD_LEVEL }).to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO blob (
                digest_algo,
                digest,
                compression_algo,
                compression_opts,
                uncompressed_len,
                compressed_len,
                bytes,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(digest_algo, digest) DO UPDATE SET
                digest = excluded.digest
            RETURNING pk
            "#,
        )
        .bind(DIGEST_ALGO_SHA256)
        .bind(digest.to_vec())
        .bind(COMPRESSION_ALGO_ZSTD)
        .bind(compression_opts)
        .bind(uncompressed_len)
        .bind(compressed_len)
        .bind(compressed)
        .bind(command.created_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let pk = row
            .try_get::<i64, _>("pk")
            .map_err(RegistryDbError::internal)?;
        Ok(BlobRef::new(pk))
    }
}

impl BlobStoreTx for super::SqliteRegistryTx<'_> {
    async fn put_blob(&mut self, command: PutBlobCommand) -> RegistryDbResult<BlobRef> {
        BlobTable::new(&mut self.tx).put(command).await
    }
}

fn to_i64(value: usize, field: &'static str) -> RegistryDbResult<i64> {
    i64::try_from(value).map_err(|_| {
        RegistryDbError::internal(anyhow::anyhow!("{field} exceeds SQLite INTEGER range"))
    })
}
