CREATE TABLE IF NOT EXISTS crawl_target (
    feed_url                 TEXT PRIMARY KEY,
    is_active                INTEGER NOT NULL,
    polling_policy_kind      TEXT,
    polling_interval_seconds INTEGER,
    updated_at               DATETIME NOT NULL
);
