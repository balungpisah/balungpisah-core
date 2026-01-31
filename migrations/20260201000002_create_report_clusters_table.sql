CREATE TABLE report_clusters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Cluster identification
    name VARCHAR(200) NOT NULL,
    description TEXT,

    -- Geographic center (centroid of member reports)
    center_lat DOUBLE PRECISION NOT NULL,
    center_lon DOUBLE PRECISION NOT NULL,

    -- Clustering radius in meters
    radius_meters INT NOT NULL DEFAULT 500,

    -- Denormalized count for quick lookup
    report_count INT NOT NULL DEFAULT 0,

    -- Status
    status cluster_status NOT NULL DEFAULT 'active',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Spatial index for proximity queries (using lat/lon box approximation)
CREATE INDEX idx_clusters_location ON report_clusters(center_lat, center_lon);
CREATE INDEX idx_clusters_status ON report_clusters(status) WHERE status = 'active';
