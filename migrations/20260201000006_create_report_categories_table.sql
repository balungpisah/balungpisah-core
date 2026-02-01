-- Create junction table for many-to-many relationship between reports and categories
-- Each report can have multiple categories, each with its own severity level

CREATE TABLE report_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    severity report_severity NOT NULL DEFAULT 'medium',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Each report can only have one entry per category
    UNIQUE(report_id, category_id)
);

-- Indexes for efficient lookups
CREATE INDEX idx_report_categories_report_id ON report_categories(report_id);
CREATE INDEX idx_report_categories_category_id ON report_categories(category_id);
CREATE INDEX idx_report_categories_severity ON report_categories(severity);

COMMENT ON TABLE report_categories IS 'Junction table linking reports to categories with per-category severity';
COMMENT ON COLUMN report_categories.severity IS 'Severity level specific to this category assignment';
