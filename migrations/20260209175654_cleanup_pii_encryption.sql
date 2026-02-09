-- ============================================================================
-- Phase 3: Cleanup PII Encryption
-- ============================================================================
-- This migration completes the encryption migration by:
-- 1. Dropping plaintext PII columns (no longer used)
-- 2. Renaming encrypted columns to original names
-- 3. Updating column comments to reflect encrypted storage
--
-- IMPORTANT: Run this ONLY after Phase 2 data migration is complete and verified
-- Prerequisites:
-- - All plaintext data has been encrypted and written to *_encrypted columns
-- - Application has been running with encrypted data for 1-2 weeks (soak period)
-- - Database backup has been taken
-- ============================================================================

-- ============================================================================
-- EXPECTATIONS TABLE
-- ============================================================================

-- Drop original plaintext email column (should be NULL everywhere after Phase 2)
ALTER TABLE expectations DROP COLUMN IF EXISTS email;

-- Rename encrypted column to original name
ALTER TABLE expectations RENAME COLUMN email_encrypted TO email;

-- Update column comment to document encryption
COMMENT ON COLUMN expectations.email IS 'AES-256-GCM encrypted email (format: nonce:ciphertext, base64)';

-- ============================================================================
-- CONTRIBUTORS TABLE
-- ============================================================================

-- Drop original plaintext PII columns (should be NULL everywhere after Phase 2)
ALTER TABLE contributors DROP COLUMN IF EXISTS email;
ALTER TABLE contributors DROP COLUMN IF EXISTS whatsapp;
ALTER TABLE contributors DROP COLUMN IF EXISTS contact_email;
ALTER TABLE contributors DROP COLUMN IF EXISTS contact_whatsapp;

-- Rename encrypted columns to original names
ALTER TABLE contributors RENAME COLUMN email_encrypted TO email;
ALTER TABLE contributors RENAME COLUMN whatsapp_encrypted TO whatsapp;
ALTER TABLE contributors RENAME COLUMN contact_email_encrypted TO contact_email;
ALTER TABLE contributors RENAME COLUMN contact_whatsapp_encrypted TO contact_whatsapp;

-- Update column comments to document encryption
COMMENT ON COLUMN contributors.email IS 'AES-256-GCM encrypted email (format: nonce:ciphertext, base64)';
COMMENT ON COLUMN contributors.whatsapp IS 'AES-256-GCM encrypted whatsapp (format: nonce:ciphertext, base64)';
COMMENT ON COLUMN contributors.contact_email IS 'AES-256-GCM encrypted contact_email (format: nonce:ciphertext, base64)';
COMMENT ON COLUMN contributors.contact_whatsapp IS 'AES-256-GCM encrypted contact_whatsapp (format: nonce:ciphertext, base64)';
