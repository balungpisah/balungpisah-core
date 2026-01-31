CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Link to ticket (1:1)
    ticket_id UUID NOT NULL UNIQUE REFERENCES tickets(id) ON DELETE CASCADE,

    -- Optional cluster membership
    cluster_id UUID REFERENCES report_clusters(id) ON DELETE SET NULL,

    -- Extracted content
    title VARCHAR(200) NOT NULL,
    description TEXT NOT NULL,

    -- Classification
    category_id UUID REFERENCES categories(id) ON DELETE SET NULL,
    severity report_severity,

    -- Additional context
    timeline TEXT,           -- When the issue started/occurred
    impact TEXT,             -- Who/how many affected

    -- Status workflow
    status report_status NOT NULL DEFAULT 'draft',

    -- Verification/resolution tracking
    verified_at TIMESTAMPTZ,
    verified_by VARCHAR(255),
    resolved_at TIMESTAMPTZ,
    resolved_by VARCHAR(255),
    resolution_notes TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_reports_ticket_id ON reports(ticket_id);
CREATE INDEX idx_reports_cluster_id ON reports(cluster_id);
CREATE INDEX idx_reports_category_id ON reports(category_id);
CREATE INDEX idx_reports_status ON reports(status);
CREATE INDEX idx_reports_severity ON reports(severity);
CREATE INDEX idx_reports_created_at ON reports(created_at DESC);

-- Pending reports for review
CREATE INDEX idx_reports_pending ON reports(status, created_at)
WHERE status IN ('draft', 'pending');
