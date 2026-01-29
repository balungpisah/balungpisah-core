-- Create contributor_role enum
CREATE TYPE contributor_role AS ENUM ('technical', 'conceptual', 'creative', 'public_voice');

-- Create experience_level enum
CREATE TYPE experience_level AS ENUM ('junior', 'mid', 'senior');

-- Create availability_level enum
CREATE TYPE availability_level AS ENUM ('observer', 'contributor', 'core_team');

-- Create contributors table for storing contributor registrations
CREATE TABLE contributors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Logto user reference
    logto_user_id VARCHAR(255) UNIQUE NOT NULL,

    -- Basic info (required)
    nickname VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    city VARCHAR(255) NOT NULL,
    role contributor_role NOT NULL,

    -- Contact (optional)
    whatsapp VARCHAR(20),

    -- Professional fields (optional, for technical/conceptual/creative roles)
    profile_url VARCHAR(500),
    current_profession VARCHAR(255),
    skills TEXT[],
    background_contribution TEXT,
    experience_level experience_level,
    availability availability_level,

    -- Public voice fields (optional, for public_voice role)
    concern_topic TEXT,
    aspiration_text TEXT,
    willing_to_beta_test BOOLEAN DEFAULT FALSE,

    -- Terms agreement
    agreed_to_terms BOOLEAN NOT NULL DEFAULT FALSE,
    agreed_to_terms_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE UNIQUE INDEX idx_contributors_email ON contributors(email);
CREATE INDEX idx_contributors_logto_user_id ON contributors(logto_user_id);
CREATE INDEX idx_contributors_role ON contributors(role);
CREATE INDEX idx_contributors_created_at ON contributors(created_at DESC);

-- Table and column comments
COMMENT ON TABLE contributors IS 'Contributors registered through the public registration form';
COMMENT ON COLUMN contributors.logto_user_id IS 'User ID from Logto authentication provider';
COMMENT ON COLUMN contributors.nickname IS 'Display name for the contributor';
COMMENT ON COLUMN contributors.email IS 'Email address (unique, used for authentication)';
COMMENT ON COLUMN contributors.city IS 'City where the contributor is located';
COMMENT ON COLUMN contributors.role IS 'Type of contribution: technical, conceptual, creative, or public_voice';
COMMENT ON COLUMN contributors.whatsapp IS 'Optional WhatsApp number for communication';
COMMENT ON COLUMN contributors.profile_url IS 'URL to professional profile (LinkedIn, portfolio, etc.)';
COMMENT ON COLUMN contributors.current_profession IS 'Current job title or profession';
COMMENT ON COLUMN contributors.skills IS 'Array of skill identifiers';
COMMENT ON COLUMN contributors.background_contribution IS 'Description of relevant background and experience';
COMMENT ON COLUMN contributors.experience_level IS 'Level of experience: junior, mid, or senior';
COMMENT ON COLUMN contributors.availability IS 'Availability level: observer, contributor, or core_team';
COMMENT ON COLUMN contributors.concern_topic IS 'Topic of concern for public voice contributors';
COMMENT ON COLUMN contributors.aspiration_text IS 'Aspirations for change (public voice contributors)';
COMMENT ON COLUMN contributors.willing_to_beta_test IS 'Whether contributor is willing to beta test features';
COMMENT ON COLUMN contributors.agreed_to_terms IS 'Whether contributor agreed to terms and conditions';
COMMENT ON COLUMN contributors.agreed_to_terms_at IS 'Timestamp when terms were agreed to';
