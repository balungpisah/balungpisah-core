-- Add fields needed for background processing
ALTER TABLE tickets
ADD COLUMN last_attempt_at TIMESTAMPTZ,
ADD COLUMN report_id UUID REFERENCES reports(id) ON DELETE SET NULL;

-- Index for retry scheduling
CREATE INDEX idx_tickets_retry ON tickets(status, last_attempt_at, retry_count)
WHERE status = 'failed' AND retry_count < 3;
