-- Global rate limit configuration table
CREATE TABLE rate_limit_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(100) UNIQUE NOT NULL,
    value INTEGER NOT NULL,
    description TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by VARCHAR(255)
);

-- Seed default configuration
INSERT INTO rate_limit_configs (key, value, description)
VALUES ('daily_ticket_limit', 5, 'Maximum tickets a user can create per day before chat is restricted');
