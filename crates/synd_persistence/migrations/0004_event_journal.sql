CREATE TABLE event_journal (
    position     INTEGER PRIMARY KEY,
    event_type   TEXT NOT NULL,
    occurred_at  DATETIME NOT NULL,
    payload_json TEXT NOT NULL CHECK (json_valid(payload_json))
);

CREATE INDEX event_journal_event_type_position_idx
    ON event_journal (event_type, position);

CREATE TABLE event_cursor (
    consumer TEXT NOT NULL PRIMARY KEY,
    position INTEGER NOT NULL CHECK (position >= 0)
);
