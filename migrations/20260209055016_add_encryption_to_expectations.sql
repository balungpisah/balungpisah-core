-- Add encrypted email column and blind index to expectations table
-- Part of field-level encryption implementation for PII protection

-- Add encrypted email column (stores nonce:ciphertext format, base64-encoded)
ALTER TABLE expectations
ADD COLUMN email_encrypted TEXT;

-- Add blind index column for email (HMAC-SHA256 hex hash, 64 characters)
-- Used for searching by email without exposing plaintext
ALTER TABLE expectations
ADD COLUMN email_index VARCHAR(64);

-- Create partial index on email_index for efficient lookups
-- Only index non-null values to save space
CREATE INDEX idx_expectations_email_index
ON expectations(email_index)
WHERE email_index IS NOT NULL;

-- Add comment to document encryption
COMMENT ON COLUMN expectations.email_encrypted IS 'AES-256-GCM encrypted email in format nonce:ciphertext (base64)';
COMMENT ON COLUMN expectations.email_index IS 'HMAC-SHA256 blind index for email search (64-char hex)';
