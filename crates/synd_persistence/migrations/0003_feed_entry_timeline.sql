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

CREATE TABLE timeline_item (
    timeline_pk INTEGER NOT NULL,
    entry_pk    INTEGER NOT NULL,
    order_time  DATETIME NOT NULL,
    created_at  DATETIME NOT NULL,
    updated_at  DATETIME NOT NULL,

    PRIMARY KEY (timeline_pk, entry_pk),
    FOREIGN KEY (timeline_pk)
        REFERENCES timeline(pk),
    FOREIGN KEY (entry_pk)
        REFERENCES entry(pk)
);

CREATE INDEX timeline_item_order_idx
    ON timeline_item(timeline_pk, order_time DESC, entry_pk);
