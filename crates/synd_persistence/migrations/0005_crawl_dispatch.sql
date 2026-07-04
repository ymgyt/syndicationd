ALTER TABLE crawl_schedule ADD COLUMN due_reason TEXT NOT NULL DEFAULT 'periodic';
ALTER TABLE crawl_schedule ADD COLUMN dispatched_at DATETIME;

CREATE INDEX crawl_schedule_dispatched_idx
    ON crawl_schedule(dispatched_at)
    WHERE dispatched_at IS NOT NULL;
