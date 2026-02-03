-- Migration: Backfill existing reports with reference_number and user_id from tickets
-- This ensures no data loss for existing reports that were created via the ticket workflow.

-- Update existing reports to copy reference_number and user_id from linked tickets
-- Only updates reports that have a ticket_id (i.e., created via old workflow)
UPDATE reports r
SET
    reference_number = t.reference_number,
    user_id = t.user_id,
    platform = t.platform,
    adk_thread_id = t.adk_thread_id
FROM tickets t
WHERE r.ticket_id = t.id
  AND r.reference_number IS NULL;

-- Log how many reports were updated (for debugging in migration logs)
DO $$
DECLARE
    updated_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO updated_count
    FROM reports
    WHERE reference_number IS NOT NULL
      AND ticket_id IS NOT NULL;

    RAISE NOTICE 'Backfilled % reports with data from tickets', updated_count;
END $$;
