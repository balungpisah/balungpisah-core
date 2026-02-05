-- Create prompts table
CREATE TABLE prompts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(200) NOT NULL UNIQUE,          -- e.g., "citizen_report_agent/system"
    name VARCHAR(200) NOT NULL,                 -- e.g., "Citizen Report Agent System Prompt"
    description TEXT,                           -- What this prompt is for
    template_content TEXT NOT NULL,             -- Jinja2 template content
    variables JSONB,                            -- Expected variables (for documentation)
    version INT NOT NULL DEFAULT 1,             -- Simple version counter
    is_active BOOLEAN NOT NULL DEFAULT TRUE,    -- Soft delete flag
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,                            -- User who created (optional for MVP)
    updated_by UUID                             -- User who last updated (optional for MVP)
);

-- Indexes
CREATE INDEX idx_prompts_key ON prompts(key);
CREATE INDEX idx_prompts_is_active ON prompts(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_prompts_created_at ON prompts(created_at DESC);

-- Add comments
COMMENT ON TABLE prompts IS 'Stores AI prompt templates with simple versioning';
COMMENT ON COLUMN prompts.key IS 'Unique identifier matching file-based template paths (e.g., citizen_report_agent/system)';
COMMENT ON COLUMN prompts.variables IS 'JSON object documenting expected template variables (e.g., {"day_name": "string", "date": "string"})';
