-- Append-only event journal: internal plumbing between command handlers and
-- processors. Compacted below the slowest cursor; not a long-term record.
-- AUTOINCREMENT keeps positions monotonic across compaction (rowid reuse
-- would rewind positions behind cursors).
CREATE TABLE event_journal (
    position     INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at  DATETIME NOT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    event_type   TEXT GENERATED ALWAYS AS (json_extract(payload_json, '$.type')) STORED NOT NULL
);

-- Interest-filtered catch-up reads (narrow interests over a large backlog).
CREATE INDEX event_journal_event_type_position_idx
    ON event_journal (event_type, position);

-- Committed read position of each event processor.
CREATE TABLE event_cursor (
    processor TEXT PRIMARY KEY,
    position  INTEGER NOT NULL
);
