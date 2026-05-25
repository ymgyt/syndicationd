CREATE TABLE feed_subscription_checked (
    subscriber_id            TEXT NOT NULL,
    feed_url                 TEXT NOT NULL,
    requirement              TEXT,
    category                 TEXT,
    refresh_policy_kind      TEXT NOT NULL,
    refresh_interval_seconds INTEGER,
    created_at               DATETIME NOT NULL,
    updated_at               DATETIME NOT NULL,

    PRIMARY KEY (subscriber_id, feed_url),
    CHECK (refresh_policy_kind IN ('manual', 'interval')),
    CHECK (
        (refresh_policy_kind = 'manual' AND refresh_interval_seconds IS NULL)
        OR
        (refresh_policy_kind = 'interval' AND refresh_interval_seconds > 0)
    )
);

INSERT INTO feed_subscription_checked (
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
    subscriber_id,
    feed_url,
    requirement,
    category,
    refresh_policy_kind,
    refresh_interval_seconds,
    created_at,
    updated_at
FROM feed_subscription;

DROP TABLE feed_subscription;

ALTER TABLE feed_subscription_checked RENAME TO feed_subscription;
