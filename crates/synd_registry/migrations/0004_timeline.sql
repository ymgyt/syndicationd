-- Read model: existence and change counter of one subscriber's reading
-- stream.
CREATE TABLE timeline (
    subscriber_id TEXT PRIMARY KEY,
    -- Monotonic change counter. Every timeline_entry mutation takes a unique
    -- seq from here; clients sync incrementally with WHERE seq > ?
    last_seq      INTEGER NOT NULL DEFAULT 0
);

-- Membership of an entry in a timeline. order_time materializes the entry's
-- canonical order at insert (immutable, keeps display cursors stable).
-- Removals stay as tombstones (deleted) so clients observe them through seq.
CREATE TABLE timeline_entry (
    subscriber_id TEXT NOT NULL,
    entry_id      TEXT NOT NULL,
    order_time    DATETIME NOT NULL,
    seq           INTEGER NOT NULL,
    deleted       INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (subscriber_id, entry_id),
    FOREIGN KEY (subscriber_id) REFERENCES timeline(subscriber_id),
    FOREIGN KEY (entry_id)      REFERENCES entry(entry_id)
);

-- Display paging: keyset scan in canonical order over live rows.
CREATE INDEX timeline_entry_order_idx
    ON timeline_entry(subscriber_id, order_time DESC, entry_id DESC)
    WHERE deleted = 0;

-- Incremental sync: WHERE seq > ?. Includes tombstones: removals must be
-- observable through sync.
CREATE INDEX timeline_entry_seq_idx
    ON timeline_entry(subscriber_id, seq);
