ALTER TABLE feed_subscription ADD COLUMN labels_json TEXT NOT NULL DEFAULT '{}'
    CHECK (json_valid(labels_json) AND json_type(labels_json) = 'object');

-- A null in a JSON merge patch omits the corresponding label.
UPDATE feed_subscription
SET labels_json = json_patch('{}', json_object(
    'category', category,
    'requirement', lower(requirement)
));

ALTER TABLE feed_subscription DROP COLUMN category;
ALTER TABLE feed_subscription DROP COLUMN requirement;

-- Use the attributes recorded in each event, including for subscriptions since removed.
UPDATE event_journal
SET payload_json = json_set(
    json_remove(payload_json, '$.attrs.category', '$.attrs.requirement'),
    '$.attrs.labels', json_patch('{}', json_object(
        'category', json_extract(payload_json, '$.attrs.category'),
        'requirement', lower(json_extract(payload_json, '$.attrs.requirement'))
    ))
)
WHERE event_type IN ('sub.feed.subscribed', 'sub.subscription.changed');
