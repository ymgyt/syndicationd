-- Declaration: the crawl instruction derived from subscriptions.
-- Boundary between the subscription world and the crawl world.
CREATE TABLE crawl_target (
    feed_pk               INTEGER PRIMARY KEY,
    state                 TEXT NOT NULL, -- 'active' | 'inactive'
    effective_policy_json TEXT CHECK (effective_policy_json IS NULL OR json_valid(effective_policy_json)),
    manual_requested_at   DATETIME,      -- pending manual crawl request

    FOREIGN KEY (feed_pk) REFERENCES feed(pk)
);

-- Observation: what crawling has learned about this feed
-- (summary of the last crawl + conditional fetch context).
-- The next crawl time is derived at runtime from crawl_target + this row;
-- schedules and inflight markers are not persisted.
CREATE TABLE crawl_state (
    feed_pk          INTEGER PRIMARY KEY,
    last_started_at  DATETIME NOT NULL,
    last_finished_at DATETIME NOT NULL,
    last_http_status INTEGER,
    last_error_kind  TEXT,
    failure_streak   INTEGER NOT NULL,
    retry_after      DATETIME,
    etag             TEXT,
    last_modified    TEXT,

    FOREIGN KEY (feed_pk) REFERENCES feed(pk)
);

-- Content-addressed store of compressed bytes (feed bodies).
-- digest is SHA-256. Identical bodies dedup to one row, so comparing
-- references (pk) compares body identity.
CREATE TABLE blob (
    pk               INTEGER PRIMARY KEY,
    digest           BLOB NOT NULL UNIQUE,
    compression_algo TEXT NOT NULL,
    uncompressed_len INTEGER NOT NULL,
    bytes            BLOB NOT NULL,
    -- Read by the compactor: unreferenced blobs older than a safety margin
    -- are deleted.
    created_at       DATETIME NOT NULL
);
