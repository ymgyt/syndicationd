CREATE TABLE IF NOT EXISTS feed_subscription (
    subscriber_id            TEXT NOT NULL,
    feed_url                 TEXT NOT NULL,
    requirement              TEXT,
    category                 TEXT,
    refresh_policy_kind      TEXT NOT NULL,
    refresh_interval_seconds INTEGER,
    created_at               DATETIME NOT NULL,
    updated_at               DATETIME NOT NULL,

    PRIMARY KEY (subscriber_id, feed_url)
);

INSERT OR IGNORE INTO feed_subscription (
    subscriber_id,
    feed_url,
    requirement,
    category,
    refresh_policy_kind,
    refresh_interval_seconds,
    created_at,
    updated_at
)
SELECT
    user_id,
    url,
    requirement,
    category,
    'interval',
    7200,
    COALESCE(created_at, CURRENT_TIMESTAMP),
    CURRENT_TIMESTAMP
FROM subscribed_feed;

CREATE TABLE IF NOT EXISTS feed_snapshot (
    feed_url      TEXT PRIMARY KEY,
    body          BLOB NOT NULL,
    content_type  TEXT,
    etag          TEXT,
    last_modified TEXT,
    fetched_at    DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS feed_refresh_state (
    feed_url            TEXT PRIMARY KEY,
    last_attempt_at     DATETIME,
    last_success_at     DATETIME,
    last_failure_at     DATETIME,
    last_error_kind     TEXT,
    last_error_message  TEXT,
    next_refresh_after  DATETIME
);
