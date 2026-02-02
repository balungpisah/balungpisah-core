-- Thread attachments table for citizen report agent
-- Stores file attachments associated with conversation threads

CREATE TABLE thread_attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id UUID NOT NULL,
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    owner_id VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_thread_file UNIQUE (thread_id, file_id)
);

-- Index for fast lookup by thread_id (most common query)
CREATE INDEX idx_thread_attachments_thread_id ON thread_attachments(thread_id);

-- Index for owner lookup (for permission checks)
CREATE INDEX idx_thread_attachments_owner ON thread_attachments(owner_id);
