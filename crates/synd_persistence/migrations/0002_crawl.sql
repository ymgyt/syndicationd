CREATE TABLE crawl_target (
    feed_endpoint_pk      INTEGER PRIMARY KEY,
    state                 TEXT NOT NULL,
    subscription_count    INTEGER NOT NULL,
    effective_policy_json TEXT,
    created_at            DATETIME NOT NULL,
    updated_at            DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk),

    CHECK (subscription_count >= 0),
    CHECK (effective_policy_json IS NULL OR json_valid(effective_policy_json))
);

CREATE TABLE crawl_schedule (
    feed_endpoint_pk  INTEGER PRIMARY KEY,
    target_updated_at DATETIME NOT NULL,
    next_crawl_after  DATETIME,
    created_at        DATETIME NOT NULL,
    updated_at        DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES crawl_target(feed_endpoint_pk)
);

CREATE INDEX crawl_schedule_due_idx
    ON crawl_schedule(next_crawl_after)
    WHERE next_crawl_after IS NOT NULL;

CREATE TABLE blob (
    pk               INTEGER PRIMARY KEY,
    digest_algo      TEXT NOT NULL,
    digest           BLOB NOT NULL,
    compression_algo TEXT NOT NULL,
    compression_opts TEXT NOT NULL,
    uncompressed_len INTEGER NOT NULL,
    compressed_len   INTEGER NOT NULL,
    bytes            BLOB NOT NULL,
    created_at       DATETIME NOT NULL,

    UNIQUE (digest_algo, digest),

    CHECK (length(digest) > 0),
    CHECK (json_valid(compression_opts)),
    CHECK (uncompressed_len >= 0),
    CHECK (compressed_len >= 0),
    CHECK (length(bytes) = compressed_len)
);

CREATE TABLE crawl_result (
    pk               INTEGER PRIMARY KEY,
    job_id           TEXT NOT NULL UNIQUE,
    feed_endpoint_pk INTEGER NOT NULL,
    started_at       DATETIME NOT NULL,
    finished_at      DATETIME NOT NULL,
    created_at       DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk)
);

CREATE TABLE crawl_http_response (
    result_pk       INTEGER PRIMARY KEY,
    status_code     INTEGER NOT NULL,
    response_url    TEXT NOT NULL,
    headers_blob_pk INTEGER NOT NULL,
    body_blob_pk    INTEGER,
    content_type    TEXT,
    content_length  INTEGER,
    etag            TEXT,
    last_modified   TEXT,
    retry_after_at  DATETIME,

    FOREIGN KEY (result_pk)
        REFERENCES crawl_result(pk),
    FOREIGN KEY (headers_blob_pk)
        REFERENCES blob(pk),
    FOREIGN KEY (body_blob_pk)
        REFERENCES blob(pk),

    CHECK (status_code BETWEEN 100 AND 999),
    CHECK (content_length IS NULL OR content_length >= 0)
);

CREATE TABLE crawl_fetch_error (
    result_pk     INTEGER PRIMARY KEY,
    error_kind    TEXT NOT NULL,
    error_message TEXT NOT NULL,

    FOREIGN KEY (result_pk)
        REFERENCES crawl_result(pk)
);

CREATE TABLE crawl_feed_parse_error (
    result_pk     INTEGER PRIMARY KEY,
    error_kind    TEXT NOT NULL,
    error_message TEXT NOT NULL,

    FOREIGN KEY (result_pk)
        REFERENCES crawl_result(pk)
);

CREATE TABLE crawl_state (
    feed_endpoint_pk INTEGER PRIMARY KEY,
    last_result_pk   INTEGER NOT NULL,
    last_started_at  DATETIME NOT NULL,
    last_finished_at DATETIME NOT NULL,
    last_http_status INTEGER,
    last_error_kind  TEXT,
    failure_streak   INTEGER NOT NULL,
    last_retry_after DATETIME,
    etag             TEXT,
    last_modified    TEXT,
    created_at       DATETIME NOT NULL,
    updated_at       DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk),
    FOREIGN KEY (last_result_pk)
        REFERENCES crawl_result(pk),

    CHECK (failure_streak >= 0),
    CHECK (last_http_status IS NULL OR last_http_status BETWEEN 100 AND 999)
);
