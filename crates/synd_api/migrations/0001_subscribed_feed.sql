CREATE TABLE subscribed_feeds (
    user_id     TEXT NOT NULL,
    feed_url    TEXT NOT NULL,
    requirement TEXT,
    category    TEXT,   
    
    PRIMARY KEY (user_id, feed_url)
);

