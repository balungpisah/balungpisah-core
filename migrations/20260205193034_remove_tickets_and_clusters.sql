-- Migration: Remove tickets feature and report_clusters functionality
-- Both superseded by direct report submission workflow
--
-- This migration:
-- 1. Removes ticket_id and cluster_id columns from reports table
-- 2. Drops tickets table and related objects
-- 3. Drops report_clusters table and related objects
-- 4. Drops associated sequences and enums
--
-- The workflow now goes directly: citizen_report_agent -> reports
-- Historical ticket and cluster data will be lost after this migration

-- Step 1: Drop indexes on columns we're removing
DROP INDEX IF EXISTS idx_reports_ticket_id;
DROP INDEX IF EXISTS idx_reports_cluster_id;

-- Step 2: Drop foreign key columns from reports table
ALTER TABLE reports DROP COLUMN IF EXISTS ticket_id CASCADE;
ALTER TABLE reports DROP COLUMN IF EXISTS cluster_id CASCADE;

-- Step 3: Drop main tables
DROP TABLE IF EXISTS tickets CASCADE;
DROP TABLE IF EXISTS report_clusters CASCADE;

-- Step 4: Drop sequences
DROP SEQUENCE IF EXISTS ticket_reference_seq CASCADE;

-- Step 5: Drop enums
DROP TYPE IF EXISTS ticket_status CASCADE;
DROP TYPE IF EXISTS cluster_status CASCADE;

-- Note: Reports table remains intact with all other fields
-- The workflow now goes directly: citizen_report_agent -> reports
