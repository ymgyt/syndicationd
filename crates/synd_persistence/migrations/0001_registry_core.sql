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
