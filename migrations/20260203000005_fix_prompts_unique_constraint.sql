-- Fix unique constraint to only apply when is_active = true
-- This allows multiple deleted (is_active = false) prompts with the same key

-- Drop the existing unique constraint
ALTER TABLE prompts DROP CONSTRAINT IF EXISTS prompts_key_key;

-- Create a partial unique index that only applies to active prompts
CREATE UNIQUE INDEX idx_prompts_key_unique_when_active
ON prompts(key)
WHERE is_active = true;

-- Add comment
COMMENT ON INDEX idx_prompts_key_unique_when_active IS 'Ensures key uniqueness only for active prompts, allowing multiple deleted prompts with same key';
