-- Drop old table and enums
DROP TABLE IF EXISTS contributors CASCADE;
DROP TYPE IF EXISTS contributor_role CASCADE;
DROP TYPE IF EXISTS experience_level CASCADE;
DROP TYPE IF EXISTS availability_level CASCADE;

-- Create simplified contributors table
-- Supports both "personal" and "organization" submissions
CREATE TABLE contributors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Submission type: "personal" or "organization"
    submission_type VARCHAR(50) NOT NULL,

    -- ============================================
    -- Personal contributor fields
    -- ============================================
    name VARCHAR(255),
    email VARCHAR(255),
    whatsapp VARCHAR(50),
    city VARCHAR(255),
    role VARCHAR(50),
    skills TEXT,
    portfolio_url TEXT,
    aspiration TEXT,

    -- ============================================
    -- Organization contributor fields
    -- ============================================
    organization_name VARCHAR(255),
    organization_type VARCHAR(50),
    contact_name VARCHAR(255),
    contact_position VARCHAR(255),
    contact_whatsapp VARCHAR(50),
    contact_email VARCHAR(255),
    contribution_offer TEXT,

    -- ============================================
    -- Common fields
    -- ============================================
    agreed BOOLEAN NOT NULL DEFAULT FALSE,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes (no unique constraints - allow multiple submissions)
CREATE INDEX idx_contributors_type ON contributors(submission_type);
CREATE INDEX idx_contributors_email ON contributors(email) WHERE email IS NOT NULL;
CREATE INDEX idx_contributors_created_at ON contributors(created_at DESC);

-- Comments
COMMENT ON TABLE contributors IS 'Contributor registrations from the public form';
COMMENT ON COLUMN contributors.submission_type IS 'Type: personal or organization';
