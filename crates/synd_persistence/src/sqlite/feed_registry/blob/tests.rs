use crate::sqlite::feed_registry::test_support::*;

#[tokio::test]
async fn blob_store_deduplicates_by_uncompressed_digest() -> anyhow::Result<()> {
    let db = migrated_db().await?;
    let created_at = Utc.with_ymd_and_hms(2026, 6, 7, 12, 0, 0).unwrap();
    let mut tx = db.begin().await?;

    let first = tx
        .put_blob(PutBlobCommand::new(b"same payload".to_vec(), created_at))
        .await?;
    let second = tx
        .put_blob(PutBlobCommand::new(b"same payload".to_vec(), created_at))
        .await?;

    assert_eq!(first, second);
    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*) AS count,
            compression_algo,
            uncompressed_len
        FROM blob
        "#,
    )
    .fetch_one(&mut *tx.tx)
    .await?;

    assert_eq!(row.try_get::<i64, _>("count")?, 1);
    assert_eq!(row.try_get::<String, _>("compression_algo")?, "zstd");
    assert_eq!(row.try_get::<i64, _>("uncompressed_len")?, 12);
    assert_eq!(tx.load_blob(first).await?, b"same payload");
    tx.commit().await?;
    Ok(())
}
