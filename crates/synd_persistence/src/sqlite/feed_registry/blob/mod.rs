use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};
use synd_registry::{
    BlobStore, RegistryDbResult,
    crawl::blob::{BlobRef, PutBlobCommand},
};

use crate::compression::{
    CompressionAlgo, CompressionCodec, DefaultCompressionCodec, StoredCompressedBytes,
};

use super::error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult};

const DEFAULT_COMPRESSION_ALGO: CompressionAlgo = CompressionAlgo::Zstd;

async fn put<C>(
    tx: &mut Transaction<'_, Sqlite>,
    compression: C,
    command: PutBlobCommand,
) -> SqliteResult<BlobRef>
where
    C: CompressionCodec,
{
    let digest = Sha256::digest(&command.bytes);
    let uncompressed_len = to_i64(command.bytes.len(), "uncompressed blob length")?;
    let compressed = compression.compress(DEFAULT_COMPRESSION_ALGO, command.bytes.as_slice())?;

    let row = sqlx::query_as::<_, PkRow>(
        r#"
            INSERT INTO blob (
                digest,
                compression_algo,
                uncompressed_len,
                bytes,
                created_at
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(digest) DO UPDATE SET
                digest = excluded.digest
            RETURNING pk
            "#,
    )
    .bind(digest.to_vec())
    .bind(DEFAULT_COMPRESSION_ALGO.as_str())
    .bind(uncompressed_len)
    .bind(compressed)
    .bind(command.created_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(BlobRef::new(row.pk))
}

async fn load<C>(
    tx: &mut Transaction<'_, Sqlite>,
    compression: C,
    blob: BlobRef,
) -> SqliteResult<Vec<u8>>
where
    C: CompressionCodec,
{
    let row = sqlx::query_as::<_, BlobRow>(
        r#"
            SELECT
                compression_algo,
                uncompressed_len,
                bytes
            FROM blob
            WHERE pk = ?
            "#,
    )
    .bind(blob.pk())
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Err(SqliteError::not_found("blob", blob.pk().to_string()));
    };
    let algo = row.compression_algo.parse::<CompressionAlgo>().decode()?;
    let uncompressed_len = to_usize(row.uncompressed_len, "uncompressed blob length")?;

    Ok(compression.decompress(
        algo,
        StoredCompressedBytes {
            bytes: row.bytes.as_slice(),
            uncompressed_len,
        },
    )?)
}

#[derive(sqlx::FromRow)]
struct PkRow {
    pk: i64,
}

#[derive(sqlx::FromRow)]
struct BlobRow {
    compression_algo: String,
    uncompressed_len: i64,
    bytes: Vec<u8>,
}

fn to_i64(value: usize, field: &'static str) -> SqliteResult<i64> {
    i64::try_from(value)
        .map_err(|_| SqliteError::decode_message(format!("{field} exceeds SQLite INTEGER range")))
}

fn to_usize(value: i64, field: &'static str) -> SqliteResult<usize> {
    usize::try_from(value)
        .map_err(|_| SqliteError::decode_message(format!("{field} must be non-negative")))
}

impl BlobStore for super::SqliteRegistryTx<'_> {
    async fn put_blob(&mut self, command: PutBlobCommand) -> RegistryDbResult<BlobRef> {
        put(&mut self.tx, DefaultCompressionCodec, command)
            .await
            .db()
    }

    async fn load_blob(&mut self, blob: BlobRef) -> RegistryDbResult<Vec<u8>> {
        load(&mut self.tx, DefaultCompressionCodec, blob).await.db()
    }
}

#[cfg(test)]
mod tests;
