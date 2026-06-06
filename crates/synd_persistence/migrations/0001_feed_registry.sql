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

CREATE TABLE crawl_job (
    pk               INTEGER PRIMARY KEY,
    job_id           TEXT NOT NULL UNIQUE,
    feed_endpoint_pk INTEGER NOT NULL,
    state            TEXT NOT NULL,
    trigger          TEXT NOT NULL,
    queue            TEXT NOT NULL,
    priority         INTEGER NOT NULL,
    run_after        DATETIME NOT NULL,
    created_at       DATETIME NOT NULL,
    updated_at       DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk),

    CHECK (length(job_id) > 0)
);

CREATE UNIQUE INDEX crawl_job_active_feed_endpoint_idx
    ON crawl_job(feed_endpoint_pk)
    WHERE state IN ('pending', 'running');

CREATE INDEX crawl_job_pending_ready_idx
    ON crawl_job(run_after, priority DESC, pk)
    WHERE state = 'pending';
