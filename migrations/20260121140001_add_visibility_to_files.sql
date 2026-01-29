-- Add visibility column and rename public_url to url in files table

-- Rename public_url column to url
ALTER TABLE files RENAME COLUMN public_url TO url;

-- Add visibility column with default 'public'
ALTER TABLE files ADD COLUMN visibility VARCHAR(20) NOT NULL DEFAULT 'public';

-- Drop old index and create new one
DROP INDEX IF EXISTS idx_files_public_url;
CREATE INDEX idx_files_url ON files(url);
CREATE INDEX idx_files_visibility ON files(visibility);
