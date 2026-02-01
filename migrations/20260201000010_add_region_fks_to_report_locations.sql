-- Add foreign key references to regional tables for report locations
-- Enables regional clustering and filtering of reports

ALTER TABLE report_locations
    ADD COLUMN province_id UUID REFERENCES provinces(id) ON DELETE SET NULL,
    ADD COLUMN regency_id UUID REFERENCES regencies(id) ON DELETE SET NULL,
    ADD COLUMN district_id UUID REFERENCES districts(id) ON DELETE SET NULL,
    ADD COLUMN village_id UUID REFERENCES villages(id) ON DELETE SET NULL;

-- Indexes for efficient regional queries
CREATE INDEX idx_report_locations_province_id ON report_locations(province_id);
CREATE INDEX idx_report_locations_regency_id ON report_locations(regency_id);
CREATE INDEX idx_report_locations_district_id ON report_locations(district_id);
CREATE INDEX idx_report_locations_village_id ON report_locations(village_id);

COMMENT ON COLUMN report_locations.province_id IS 'Reference to province for regional clustering';
COMMENT ON COLUMN report_locations.regency_id IS 'Reference to regency/city for regional clustering';
COMMENT ON COLUMN report_locations.district_id IS 'Reference to district/kecamatan for regional clustering';
COMMENT ON COLUMN report_locations.village_id IS 'Reference to village/kelurahan for regional clustering';
