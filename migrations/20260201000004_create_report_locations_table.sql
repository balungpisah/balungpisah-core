CREATE TABLE report_locations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Link to report (1:1)
    report_id UUID NOT NULL UNIQUE REFERENCES reports(id) ON DELETE CASCADE,

    -- Original input from user
    raw_input TEXT NOT NULL,

    -- Nominatim response data
    display_name TEXT,
    lat DOUBLE PRECISION,
    lon DOUBLE PRECISION,

    -- OpenStreetMap reference
    osm_id BIGINT,
    osm_type VARCHAR(20),  -- 'node', 'way', 'relation'

    -- Address components (from Nominatim)
    road VARCHAR(255),
    neighbourhood VARCHAR(255),
    suburb VARCHAR(255),
    city VARCHAR(255),
    state VARCHAR(255),
    postcode VARCHAR(20),
    country_code VARCHAR(2),

    -- Bounding box for the location
    bounding_box JSONB,  -- [minLat, maxLat, minLon, maxLon]

    -- Geocoding metadata
    geocoding_source geocoding_source NOT NULL DEFAULT 'nominatim',
    geocoding_score DECIMAL(3,2),  -- Confidence 0.00-1.00
    geocoded_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Spatial index for clustering queries
CREATE INDEX idx_report_locations_coords ON report_locations(lat, lon)
WHERE lat IS NOT NULL AND lon IS NOT NULL;

CREATE INDEX idx_report_locations_city ON report_locations(city);
CREATE INDEX idx_report_locations_suburb ON report_locations(suburb);
