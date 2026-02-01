-- Remove deprecated category_id and severity columns from reports table
-- Data has been migrated to report_categories junction table

-- Drop the foreign key constraint index first
DROP INDEX IF EXISTS idx_reports_category_id;
DROP INDEX IF EXISTS idx_reports_severity;

-- Drop the columns
ALTER TABLE reports DROP COLUMN IF EXISTS category_id;
ALTER TABLE reports DROP COLUMN IF EXISTS severity;
