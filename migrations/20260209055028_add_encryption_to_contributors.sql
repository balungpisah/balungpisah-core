-- Add encrypted PII columns and blind indexes to contributors table
-- Part of field-level encryption implementation for PII protection

-- Add encrypted columns for all PII fields (stores nonce:ciphertext format, base64-encoded)
ALTER TABLE contributors
ADD COLUMN email_encrypted TEXT,
ADD COLUMN whatsapp_encrypted TEXT,
ADD COLUMN contact_email_encrypted TEXT,
ADD COLUMN contact_whatsapp_encrypted TEXT;

-- Add blind index columns for all PII fields (HMAC-SHA256 hex hash, 64 characters)
-- Used for searching by these fields without exposing plaintext
ALTER TABLE contributors
ADD COLUMN email_index VARCHAR(64),
ADD COLUMN whatsapp_index VARCHAR(64),
ADD COLUMN contact_email_index VARCHAR(64),
ADD COLUMN contact_whatsapp_index VARCHAR(64);

-- Create partial indexes on all blind index columns for efficient lookups
-- Only index non-null values to save space
CREATE INDEX idx_contributors_email_index
ON contributors(email_index)
WHERE email_index IS NOT NULL;

CREATE INDEX idx_contributors_whatsapp_index
ON contributors(whatsapp_index)
WHERE whatsapp_index IS NOT NULL;

CREATE INDEX idx_contributors_contact_email_index
ON contributors(contact_email_index)
WHERE contact_email_index IS NOT NULL;

CREATE INDEX idx_contributors_contact_whatsapp_index
ON contributors(contact_whatsapp_index)
WHERE contact_whatsapp_index IS NOT NULL;

-- Add comments to document encryption
COMMENT ON COLUMN contributors.email_encrypted IS 'AES-256-GCM encrypted email in format nonce:ciphertext (base64)';
COMMENT ON COLUMN contributors.whatsapp_encrypted IS 'AES-256-GCM encrypted whatsapp in format nonce:ciphertext (base64)';
COMMENT ON COLUMN contributors.contact_email_encrypted IS 'AES-256-GCM encrypted contact_email in format nonce:ciphertext (base64)';
COMMENT ON COLUMN contributors.contact_whatsapp_encrypted IS 'AES-256-GCM encrypted contact_whatsapp in format nonce:ciphertext (base64)';

COMMENT ON COLUMN contributors.email_index IS 'HMAC-SHA256 blind index for email search (64-char hex)';
COMMENT ON COLUMN contributors.whatsapp_index IS 'HMAC-SHA256 blind index for whatsapp search (64-char hex)';
COMMENT ON COLUMN contributors.contact_email_index IS 'HMAC-SHA256 blind index for contact_email search (64-char hex)';
COMMENT ON COLUMN contributors.contact_whatsapp_index IS 'HMAC-SHA256 blind index for contact_whatsapp search (64-char hex)';
