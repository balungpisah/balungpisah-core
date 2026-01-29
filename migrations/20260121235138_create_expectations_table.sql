-- Create expectations table for storing user expectations from landing page
CREATE TABLE expectations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255),
    email VARCHAR(255),
    expectation TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_expectations_email ON expectations(email) WHERE email IS NOT NULL;
CREATE INDEX idx_expectations_created_at ON expectations(created_at DESC);

COMMENT ON TABLE expectations IS 'User expectations submitted from the landing page';
COMMENT ON COLUMN expectations.name IS 'Optional name of the person submitting';
COMMENT ON COLUMN expectations.email IS 'Optional email for follow-up communication';
COMMENT ON COLUMN expectations.expectation IS 'The user expectation/harapan text (required)';
