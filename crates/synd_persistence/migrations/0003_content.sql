-- Observation: the feed as observed by the latest crawl. No row = never
-- observed as a feed. Replaced in place; crawl history is not kept.
CREATE TABLE feed_snapshot (
    feed_pk      INTEGER PRIMARY KEY,
    meta_json    TEXT NOT NULL CHECK (json_valid(meta_json)),
    -- Source body of this snapshot. Comparing references detects body
    -- change, which gates entry projection.
    body_blob_pk INTEGER NOT NULL,

    FOREIGN KEY (feed_pk)      REFERENCES feed(pk),
    FOREIGN KEY (body_blob_pk) REFERENCES blob(pk)
);

-- Observation: an entry as observed by crawling. Identity and observation
-- share one table because an entry only exists once observed.
CREATE TABLE entry (
    entry_id   TEXT PRIMARY KEY,
    feed_pk    INTEGER NOT NULL,
    attrs_json TEXT NOT NULL CHECK (json_valid(attrs_json)),
    -- Full content. Kept out of attrs_json so hot queries do not carry it.
    content    TEXT,
    -- Materialized EntryOrderKey: published, else updated, else first seen;
    -- fixed at discovery. First component of the canonical entry order
    -- (order_time DESC, entry_id DESC) shared by every view and cursor.
    order_time DATETIME NOT NULL,

    FOREIGN KEY (feed_pk) REFERENCES feed(pk)
);

-- Per-feed entry access in canonical order (timeline catchup, per-feed
-- browsing, latest-entry lookups).
CREATE INDEX entry_feed_order_idx
    ON entry(feed_pk, order_time DESC, entry_id DESC);
