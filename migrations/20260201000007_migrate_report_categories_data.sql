-- Migrate existing category_id and severity data from reports to report_categories junction table

INSERT INTO report_categories (report_id, category_id, severity)
SELECT id, category_id, COALESCE(severity, 'medium')
FROM reports
WHERE category_id IS NOT NULL;

COMMENT ON TABLE report_categories IS 'Junction table linking reports to categories with per-category severity (migrated from reports.category_id)';
