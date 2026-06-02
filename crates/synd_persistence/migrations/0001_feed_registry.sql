CREATE TABLE feed_endpoint (
    pk         INTEGER PRIMARY KEY,
    url        TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE TABLE feed_endpoint_subscription (
    subscriber_id      TEXT NOT NULL,
    feed_endpoint_pk   INTEGER NOT NULL,
    requirement        TEXT,
    category           TEXT,
    crawl_policy_json  TEXT NOT NULL CHECK (json_valid(crawl_policy_json)),
    created_at         DATETIME NOT NULL,
    updated_at         DATETIME NOT NULL,

    PRIMARY KEY (subscriber_id, feed_endpoint_pk),
    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk)
);

CREATE TABLE crawl_target (
    feed_endpoint_pk      INTEGER PRIMARY KEY,
    state                 TEXT NOT NULL,
    subscription_count    INTEGER NOT NULL,
    effective_policy_json TEXT,
    created_at            DATETIME NOT NULL,
    updated_at            DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk),

    CHECK (state IN ('active', 'inactive')),
    CHECK (subscription_count >= 0),
    CHECK (
        (
            state = 'active'
            AND subscription_count > 0
            AND effective_policy_json IS NOT NULL
            AND json_valid(effective_policy_json)
        )
        OR
        (
            state = 'inactive'
            AND subscription_count = 0
            AND effective_policy_json IS NULL
        )
    )
);
