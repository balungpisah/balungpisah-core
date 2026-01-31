CREATE TABLE tickets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Link to ADK conversation (cross-database soft reference)
    adk_thread_id UUID NOT NULL,

    -- Link to citizen (from auth)
    user_id VARCHAR(255) NOT NULL,

    -- Human-readable reference
    reference_number VARCHAR(20) NOT NULL UNIQUE,

    -- Chat platform info
    platform VARCHAR(50) NOT NULL DEFAULT 'web',

    -- Agent's simple confidence (0.0-1.0) - "I think I have enough info"
    confidence_score DECIMAL(3,2) NOT NULL DEFAULT 0.50,

    -- Populated by background job (Phase 2)
    completeness_score DECIMAL(3,2),
    missing_fields JSONB,
    preliminary_data JSONB,

    -- Status
    status ticket_status NOT NULL DEFAULT 'submitted',
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,

    -- Error tracking (Phase 2)
    error_message TEXT,
    retry_count INT NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_tickets_adk_thread_id ON tickets(adk_thread_id);
CREATE INDEX idx_tickets_user_id ON tickets(user_id);
CREATE INDEX idx_tickets_reference_number ON tickets(reference_number);
CREATE INDEX idx_tickets_status ON tickets(status);
CREATE INDEX idx_tickets_created_at ON tickets(created_at DESC);
CREATE INDEX idx_tickets_pending ON tickets(status) WHERE status = 'submitted';

-- Sequence for reference numbers
CREATE SEQUENCE ticket_reference_seq START 1;
