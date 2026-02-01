-- Create report_tag_type enum for categorizing report types
CREATE TYPE report_tag_type AS ENUM (
    'report',       -- General report/observation
    'proposal',     -- Proposal for improvement
    'complaint',    -- Complaint about an issue
    'inquiry',      -- Question or request for information
    'appreciation'  -- Appreciation or positive feedback
);

-- Create report_tags table for tagging reports
CREATE TABLE report_tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    tag_type report_tag_type NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Each report can only have one entry per tag type
    UNIQUE(report_id, tag_type)
);

-- Indexes for efficient lookups
CREATE INDEX idx_report_tags_report_id ON report_tags(report_id);
CREATE INDEX idx_report_tags_tag_type ON report_tags(tag_type);

COMMENT ON TABLE report_tags IS 'Tags for classifying report types (report, proposal, complaint, etc.)';
COMMENT ON COLUMN report_tags.tag_type IS 'The type of tag assigned to the report';
