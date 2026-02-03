-- Migration: Create report_attachments junction table
-- Links file attachments to reports (copied from thread_attachments during processing)

CREATE TABLE report_attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Ensure no duplicate attachments per report
    UNIQUE(report_id, file_id)
);

-- Index for efficient lookups by report_id
CREATE INDEX idx_report_attachments_report_id ON report_attachments(report_id);

-- Index for reverse lookup by file_id
CREATE INDEX idx_report_attachments_file_id ON report_attachments(file_id);

COMMENT ON TABLE report_attachments IS 'Junction table linking file attachments to reports';
COMMENT ON COLUMN report_attachments.report_id IS 'Reference to the report';
COMMENT ON COLUMN report_attachments.file_id IS 'Reference to the file attachment';
