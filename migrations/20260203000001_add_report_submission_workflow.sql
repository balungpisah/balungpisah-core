-- Migration: Add report submission workflow
-- This enables citizens to submit reports directly without going through tickets first.
-- Reports can now be created with minimal data and enriched via background processing.

-- 1. Create sequence for report reference numbers
CREATE SEQUENCE report_reference_seq START 1;

-- 2. Create enum for report job status
CREATE TYPE report_job_status AS ENUM ('submitted', 'processing', 'completed', 'failed');

-- 3. Add new columns to reports table for direct submission
ALTER TABLE reports
ADD COLUMN reference_number VARCHAR(50) UNIQUE,
ADD COLUMN adk_thread_id UUID,
ADD COLUMN user_id VARCHAR(255),
ADD COLUMN platform VARCHAR(50) DEFAULT 'web';

-- 4. Make ticket_id, title, description nullable for initial pending state
ALTER TABLE reports ALTER COLUMN ticket_id DROP NOT NULL;
ALTER TABLE reports ALTER COLUMN title DROP NOT NULL;
ALTER TABLE reports ALTER COLUMN description DROP NOT NULL;

-- 5. Create report_jobs table for background processing state
CREATE TABLE report_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Link to report
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,

    -- Processing status
    status report_job_status NOT NULL DEFAULT 'submitted',

    -- Agent confidence score (0.0-1.0)
    confidence_score NUMERIC(3,2),

    -- Retry tracking
    retry_count INT NOT NULL DEFAULT 0,
    error_message TEXT,

    -- Timestamps
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    last_attempt_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 6. Add indexes for report_jobs
CREATE INDEX idx_report_jobs_report_id ON report_jobs(report_id);
CREATE INDEX idx_report_jobs_status ON report_jobs(status);
CREATE INDEX idx_report_jobs_submitted ON report_jobs(status, last_attempt_at, retry_count)
WHERE status IN ('submitted', 'processing');

-- 7. Add indexes for new reports columns
CREATE INDEX idx_reports_reference_number ON reports(reference_number);
CREATE INDEX idx_reports_adk_thread_id ON reports(adk_thread_id);
CREATE INDEX idx_reports_user_id ON reports(user_id);

-- 8. Add comment explaining the workflow
COMMENT ON TABLE report_jobs IS 'Background job queue for processing citizen report submissions. Tracks extraction and enrichment status.';
COMMENT ON COLUMN reports.reference_number IS 'Human-readable reference number for citizen tracking (format: RPT-YYYY-NNNNNNN)';
COMMENT ON COLUMN reports.adk_thread_id IS 'Reference to ADK conversation thread for extraction';
COMMENT ON COLUMN reports.user_id IS 'Citizen account ID who submitted the report';
