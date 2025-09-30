CREATE TABLE subscribed_feed (
    user_id     TEXT NOT NULL,
    url         TEXT NOT NULL,
    requirement TEXT,
    category    TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (user_id, url)
);

