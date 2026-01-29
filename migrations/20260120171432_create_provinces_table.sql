-- Create provinces table for Indonesian administrative regions (level 1)
CREATE TABLE provinces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(2) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    lat DOUBLE PRECISION,
    lng DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_provinces_code ON provinces(code);
CREATE INDEX idx_provinces_name ON provinces(name);

COMMENT ON TABLE provinces IS 'Indonesian provinces (provinsi) - administrative level 1';
COMMENT ON COLUMN provinces.code IS 'BPS province code (2 digits)';
COMMENT ON COLUMN provinces.lat IS 'Latitude coordinate of province centroid';
COMMENT ON COLUMN provinces.lng IS 'Longitude coordinate of province centroid';
