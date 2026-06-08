use sha2::{Digest, Sha256};
use sqlx::{Row, Sqlite, Transaction};
use synd_registry::{
    BlobStoreTx, RegistryDbError, RegistryDbResult,
    crawl::blob::{BlobRef, PutBlobCommand},
};

use crate::compression::{
    CompressionAlgo, CompressionCodec, DefaultCompressionCodec, StoredCompressedBytes,
};

const DIGEST_ALGO_SHA256: &str = "sha256";
const DEFAULT_COMPRESSION_ALGO: CompressionAlgo = CompressionAlgo::Zstd;

pub(super) struct BlobTable<'tx, 'db, C> {
    tx: &'tx mut Transaction<'db, Sqlite>,
    compression: C,
}

impl<'tx, 'db, C> BlobTable<'tx, 'db, C>
where
    C: CompressionCodec,
{
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>, compression: C) -> Self {
        Self { tx, compression }
    }

    pub(super) async fn put(&mut self, command: PutBlobCommand) -> RegistryDbResult<BlobRef> {
        let digest = Sha256::digest(&command.bytes);
        let uncompressed_len = to_i64(command.bytes.len(), "uncompressed blob length")?;
        let compressed = self
            .compression
            .compress(DEFAULT_COMPRESSION_ALGO, command.bytes.as_slice())
            .map_err(RegistryDbError::internal)?;
        let compressed_len = to_i64(compressed.len(), "compressed blob length")?;

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
        .bind(DEFAULT_COMPRESSION_ALGO.as_str())
        .bind(compressed.opts_json())
        .bind(uncompressed_len)
        .bind(compressed_len)
        .bind(compressed.bytes())
        .bind(command.created_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let pk = row
            .try_get::<i64, _>("pk")
            .map_err(RegistryDbError::internal)?;
        Ok(BlobRef::new(pk))
    }

    pub(super) async fn load(&mut self, blob: BlobRef) -> RegistryDbResult<Vec<u8>> {
        let row = sqlx::query(
            r#"
            SELECT
                compression_algo,
                compression_opts,
                uncompressed_len,
                bytes
            FROM blob
            WHERE pk = ?
            "#,
        )
        .bind(blob.pk())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "blob not found: {}",
                blob.pk()
            )));
        };
        let algo = row
            .try_get::<String, _>("compression_algo")
            .map_err(RegistryDbError::internal)?
            .parse::<CompressionAlgo>()
            .map_err(RegistryDbError::internal)?;
        let opts_json = row
            .try_get::<String, _>("compression_opts")
            .map_err(RegistryDbError::internal)?;
        let bytes = row
            .try_get::<Vec<u8>, _>("bytes")
            .map_err(RegistryDbError::internal)?;
        let uncompressed_len = row
            .try_get::<i64, _>("uncompressed_len")
            .map_err(RegistryDbError::internal)
            .and_then(|value| to_usize(value, "uncompressed blob length"))?;

        self.compression
            .decompress(
                algo,
                StoredCompressedBytes {
                    opts_json: &opts_json,
                    bytes: bytes.as_slice(),
                    uncompressed_len,
                },
            )
            .map_err(RegistryDbError::internal)
    }
}

impl BlobStoreTx for super::SqliteRegistryTx<'_> {
    async fn put_blob(&mut self, command: PutBlobCommand) -> RegistryDbResult<BlobRef> {
        BlobTable::new(&mut self.tx, DefaultCompressionCodec)
            .put(command)
            .await
    }

    async fn load_blob(&mut self, blob: BlobRef) -> RegistryDbResult<Vec<u8>> {
        BlobTable::new(&mut self.tx, DefaultCompressionCodec)
            .load(blob)
            .await
    }
}

fn to_i64(value: usize, field: &'static str) -> RegistryDbResult<i64> {
    i64::try_from(value).map_err(|_| {
        RegistryDbError::internal(anyhow::anyhow!("{field} exceeds SQLite INTEGER range"))
    })
}

fn to_usize(value: i64, field: &'static str) -> RegistryDbResult<usize> {
    usize::try_from(value)
        .map_err(|_| RegistryDbError::internal(anyhow::anyhow!("{field} must be non-negative")))
}
