-- Create regencies table for Indonesian administrative regions (level 2)
-- Includes both kabupaten (regencies) and kota (cities)
CREATE TABLE regencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(5) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    lat DOUBLE PRECISION,
    lng DOUBLE PRECISION,
    province_id UUID NOT NULL REFERENCES provinces(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_regencies_code ON regencies(code);
CREATE INDEX idx_regencies_name ON regencies(name);
CREATE INDEX idx_regencies_province_id ON regencies(province_id);

COMMENT ON TABLE regencies IS 'Indonesian regencies/cities (kabupaten/kota) - administrative level 2';
COMMENT ON COLUMN regencies.code IS 'BPS regency code (format: XX.XX where XX is province code)';
COMMENT ON COLUMN regencies.lat IS 'Latitude coordinate of regency centroid';
COMMENT ON COLUMN regencies.lng IS 'Longitude coordinate of regency centroid';
