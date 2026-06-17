CREATE TABLE event_journal (
    position     INTEGER PRIMARY KEY,
    occurred_at  DATETIME NOT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
    event_type   TEXT GENERATED ALWAYS AS (json_extract(payload_json, '$.type')) STORED NOT NULL
);

CREATE INDEX event_journal_event_type_position_idx
    ON event_journal (event_type, position);

CREATE TABLE event_cursor (
    consumer TEXT NOT NULL PRIMARY KEY,
    position INTEGER NOT NULL CHECK (position >= 0)
);
