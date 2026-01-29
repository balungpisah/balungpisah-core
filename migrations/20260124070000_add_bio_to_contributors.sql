-- Add bio field for personal contributors to describe their background/experience
ALTER TABLE contributors ADD COLUMN bio TEXT;

COMMENT ON COLUMN contributors.bio IS 'Personal contributor background/experience description';
