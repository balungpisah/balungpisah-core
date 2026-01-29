-- Create villages table for Indonesian administrative regions (level 4)
-- Includes both kelurahan (urban villages) and desa (rural villages)
CREATE TABLE villages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(13) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    lat DOUBLE PRECISION,
    lng DOUBLE PRECISION,
    district_id UUID NOT NULL REFERENCES districts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_villages_code ON villages(code);
CREATE INDEX idx_villages_name ON villages(name);
CREATE INDEX idx_villages_district_id ON villages(district_id);

COMMENT ON TABLE villages IS 'Indonesian villages (kelurahan/desa) - administrative level 4';
COMMENT ON COLUMN villages.code IS 'BPS village code (format: XX.XX.XX.XXXX)';
COMMENT ON COLUMN villages.lat IS 'Latitude coordinate of village centroid';
COMMENT ON COLUMN villages.lng IS 'Longitude coordinate of village centroid';
