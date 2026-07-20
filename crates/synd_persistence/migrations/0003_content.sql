-- Observation: the feed as observed by the latest crawl. No row = never
-- observed as a feed. Replaced in place; crawl history is not kept.
CREATE TABLE feed_snapshot (
    feed_pk      INTEGER PRIMARY KEY,
    meta_json    TEXT NOT NULL CHECK (json_valid(meta_json)),
    -- Source body retained as provenance for the parsed snapshot.
    body_blob_pk INTEGER NOT NULL,

    FOREIGN KEY (feed_pk)      REFERENCES feed(pk),
    FOREIGN KEY (body_blob_pk) REFERENCES blob(pk)
);

-- Observation: an entry as observed by crawling. Identity and observation
-- share one table because an entry only exists once observed.
CREATE TABLE entry (
    entry_id   TEXT PRIMARY KEY,
    feed_pk    INTEGER NOT NULL,
    entry_json TEXT NOT NULL CHECK (json_valid(entry_json)),
    -- Materialized EntryOrderKey: published, else updated, else first seen;
    -- fixed at discovery. First component of the canonical entry order
    -- (order_time DESC, entry_id DESC) shared by every view and cursor.
    order_time DATETIME NOT NULL,

    UNIQUE (feed_pk, entry_id),
    FOREIGN KEY (feed_pk) REFERENCES feed(pk)
);

-- Per-feed entry access in canonical order (timeline catchup, per-feed
-- browsing, latest-entry lookups).
CREATE INDEX entry_feed_order_idx
    ON entry(feed_pk, order_time DESC, entry_id DESC);

-- Entries declared by the latest accepted body of each feed. Entry catalog
-- rows remain after membership disappears because timelines retain history.
CREATE TABLE feed_entry (
    feed_pk  INTEGER NOT NULL,
    entry_id TEXT NOT NULL,

    PRIMARY KEY (feed_pk, entry_id),
    FOREIGN KEY (feed_pk, entry_id)
        REFERENCES entry(feed_pk, entry_id)
);
