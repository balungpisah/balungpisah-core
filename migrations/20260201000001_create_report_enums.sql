-- Report status workflow
CREATE TYPE report_status AS ENUM (
    'draft',        -- Extracted but not verified
    'pending',      -- Awaiting review
    'verified',     -- Confirmed valid report
    'in_progress',  -- Being addressed
    'resolved',     -- Issue resolved
    'rejected'      -- Invalid/duplicate/spam
);

-- Report severity levels
CREATE TYPE report_severity AS ENUM (
    'low',       -- Minor inconvenience
    'medium',    -- Needs attention
    'high',      -- Urgent
    'critical'   -- Safety hazard / emergency
);

-- Geocoding source tracking
CREATE TYPE geocoding_source AS ENUM (
    'nominatim',  -- OpenStreetMap Nominatim API
    'manual',     -- Manually entered coordinates
    'fallback'    -- Used regions table fallback
);

-- Cluster status
CREATE TYPE cluster_status AS ENUM (
    'active',     -- Accepting new reports
    'monitoring', -- Under observation
    'resolved',   -- All reports resolved
    'archived'    -- Historical
);
