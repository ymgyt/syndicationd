#![allow(clippy::needless_raw_string_hashes)]

use sqlx::{Sqlite, Transaction};
use synd_registry::{CommitTx, FeedRegistryDb, RegistryDbError, RegistryDbResult};

use self::error::{IntoDbResult, SqliteResult};
use super::SqliteDatabase;

mod blob;
mod codec;
mod crawl;
mod entry;
mod error;
mod feed;
mod feed_endpoint;
mod journal;
mod pagination;
mod subscription;
#[cfg(test)]
mod test_support;
mod timeline;

/// SQLite-backed registry database handle.
#[derive(Clone)]
pub struct SqliteFeedRegistryDb {
    db: SqliteDatabase,
}

/// `SQLite` transaction used to atomically update registry state and event progress.
pub struct SqliteRegistryTx<'a> {
    tx: Transaction<'a, Sqlite>,
}

impl SqliteFeedRegistryDb {
    pub fn new(db: SqliteDatabase) -> Self {
        Self { db }
    }
}

impl FeedRegistryDb for SqliteFeedRegistryDb {
    type Tx<'a> = SqliteRegistryTx<'a>;

    async fn begin(&self) -> Result<Self::Tx<'_>, RegistryDbError> {
        let tx = self.db.begin().await?;
        Ok(SqliteRegistryTx { tx })
    }
}

impl CommitTx for SqliteRegistryTx<'_> {
    async fn commit(self) -> RegistryDbResult<()> {
        commit_tx(self.tx).await.db()
    }
}

async fn commit_tx(tx: Transaction<'_, Sqlite>) -> SqliteResult<()> {
    tx.commit().await?;
    Ok(())
}
