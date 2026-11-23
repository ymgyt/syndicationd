ALTER TABLE timeline RENAME TO old_timeline;
ALTER TABLE timeline_entry RENAME TO old_timeline_entry;

CREATE TABLE timeline (
    timeline_id   BLOB NOT NULL PRIMARY KEY,
    subscriber_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    last_seq      INTEGER NOT NULL DEFAULT 0,
    UNIQUE (subscriber_id, name)
);

CREATE TABLE timeline_entry (
    timeline_id BLOB NOT NULL REFERENCES timeline(timeline_id),
    entry_id    TEXT NOT NULL REFERENCES entry(entry_id),
    order_time  DATETIME NOT NULL,
    seq         INTEGER NOT NULL,
    deleted     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (timeline_id, entry_id)
);

-- Give each existing stream a UUID v4, setting the version and variant nibbles.
INSERT INTO timeline (timeline_id, subscriber_id, name, last_seq)
SELECT unhex(hex(randomblob(6)) || '4' || substr(hex(randomblob(2)), 2)
             || substr('89ab', 1 + (random() & 3), 1) || substr(hex(randomblob(8)), 2)),
       subscriber_id, 'all', last_seq
FROM old_timeline;

INSERT INTO timeline_entry (timeline_id, entry_id, order_time, seq, deleted)
SELECT t.timeline_id, e.entry_id, e.order_time, e.seq, e.deleted
FROM old_timeline_entry AS e
JOIN timeline AS t ON t.subscriber_id = e.subscriber_id;

DROP TABLE old_timeline_entry;
DROP TABLE old_timeline;

CREATE INDEX timeline_entry_order_idx
    ON timeline_entry(timeline_id, order_time DESC, entry_id DESC)
    WHERE deleted = 0;
CREATE INDEX timeline_entry_seq_idx ON timeline_entry(timeline_id, seq);
