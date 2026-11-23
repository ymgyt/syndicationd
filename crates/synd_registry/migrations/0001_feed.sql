-- Ledger: the fact that this URL is treated as a feed.
-- Row existence means the registry manages the URL. Immutable.
CREATE TABLE feed (
    pk  INTEGER PRIMARY KEY,
    url TEXT NOT NULL UNIQUE
);

-- Declaration: on what terms a subscriber reads a feed.
-- Editing terms keeps subscribed_at; unsubscribe deletes the row.
CREATE TABLE feed_subscription (
    subscriber_id     TEXT NOT NULL,
    feed_pk           INTEGER NOT NULL,
    requirement       TEXT,
    category          TEXT,
    crawl_policy_json TEXT NOT NULL CHECK (json_valid(crawl_policy_json)),
    subscribed_at     DATETIME NOT NULL,

    PRIMARY KEY (subscriber_id, feed_pk),
    FOREIGN KEY (feed_pk) REFERENCES feed(pk)
);
