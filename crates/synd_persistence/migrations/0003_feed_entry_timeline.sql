CREATE TABLE feed (
    pk                       INTEGER PRIMARY KEY,
    feed_endpoint_pk         INTEGER NOT NULL UNIQUE,
    current_meta_json        TEXT NOT NULL CHECK (json_valid(current_meta_json)),
    current_body_blob_pk     INTEGER NOT NULL,
    current_source_result_pk INTEGER NOT NULL,
    first_seen_at            DATETIME NOT NULL,
    last_seen_at             DATETIME NOT NULL,
    updated_at               DATETIME NOT NULL,

    FOREIGN KEY (feed_endpoint_pk)
        REFERENCES feed_endpoint(pk),
    FOREIGN KEY (current_body_blob_pk)
        REFERENCES blob(pk),
    FOREIGN KEY (current_source_result_pk)
        REFERENCES crawl_result(pk)
);

CREATE TABLE entry (
    pk                       INTEGER PRIMARY KEY,
    feed_pk                  INTEGER NOT NULL,
    entry_id                 TEXT NOT NULL UNIQUE,
    current_content_json     TEXT NOT NULL CHECK (json_valid(current_content_json)),
    current_order_time       DATETIME NOT NULL,
    current_source_result_pk INTEGER NOT NULL,
    first_seen_at            DATETIME NOT NULL,
    last_seen_at             DATETIME NOT NULL,
    updated_at               DATETIME NOT NULL,

    FOREIGN KEY (feed_pk)
        REFERENCES feed(pk),
    FOREIGN KEY (current_source_result_pk)
        REFERENCES crawl_result(pk),

    CHECK (length(entry_id) > 0)
);

CREATE INDEX entry_feed_order_idx
    ON entry(feed_pk, current_order_time DESC, entry_id);

CREATE TABLE timeline (
    pk              INTEGER PRIMARY KEY,
    subscriber_id   TEXT NOT NULL,
    kind            TEXT NOT NULL,
    name            TEXT,
    definition_json TEXT CHECK (definition_json IS NULL OR json_valid(definition_json)),
    -- Monotonic change counter. Every timeline_entry mutation takes its seq
    -- from here, so clients sync incrementally with WHERE seq > ?
    last_seq        INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL,
    updated_at      DATETIME NOT NULL,

    CHECK (length(subscriber_id) > 0),
    CHECK (length(kind) > 0)
);

CREATE UNIQUE INDEX timeline_default_subscriber_idx
    ON timeline(subscriber_id)
    WHERE kind = 'default';

CREATE INDEX timeline_subscriber_idx
    ON timeline(subscriber_id, pk);

-- Display order (entry_id, order_time) is write-once: copied from entry at
-- insert and never updated, so pagination cursors over (order_time, entry_id)
-- stay stable.
-- seq records the change that last touched the row. Removed rows stay as
-- tombstones (deleted_at) so removals are observable through seq.
CREATE TABLE timeline_entry (
    timeline_pk INTEGER NOT NULL,
    entry_pk    INTEGER NOT NULL,
    entry_id    TEXT NOT NULL,
    order_time  DATETIME NOT NULL,
    seq         INTEGER NOT NULL,
    deleted_at  DATETIME,
    created_at  DATETIME NOT NULL,

    PRIMARY KEY (timeline_pk, entry_pk),
    FOREIGN KEY (timeline_pk)
        REFERENCES timeline(pk),
    FOREIGN KEY (entry_pk)
        REFERENCES entry(pk),

    CHECK (length(entry_id) > 0)
);

-- Serves the display page query over live rows:
-- ORDER BY order_time DESC, entry_id DESC with a keyset cursor over the same pair.
CREATE INDEX timeline_entry_order_idx
    ON timeline_entry(timeline_pk, order_time DESC, entry_id DESC)
    WHERE deleted_at IS NULL;

-- Serves the sync query: WHERE seq > ?
CREATE INDEX timeline_entry_seq_idx
    ON timeline_entry(timeline_pk, seq);
