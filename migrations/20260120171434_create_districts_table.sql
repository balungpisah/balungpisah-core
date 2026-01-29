-- Create districts table for Indonesian administrative regions (level 3)
CREATE TABLE districts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(8) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    lat DOUBLE PRECISION,
    lng DOUBLE PRECISION,
    regency_id UUID NOT NULL REFERENCES regencies(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_districts_code ON districts(code);
CREATE INDEX idx_districts_name ON districts(name);
CREATE INDEX idx_districts_regency_id ON districts(regency_id);

COMMENT ON TABLE districts IS 'Indonesian districts (kecamatan) - administrative level 3';
COMMENT ON COLUMN districts.code IS 'BPS district code (format: XX.XX.XX)';
COMMENT ON COLUMN districts.lat IS 'Latitude coordinate of district centroid';
COMMENT ON COLUMN districts.lng IS 'Longitude coordinate of district centroid';
