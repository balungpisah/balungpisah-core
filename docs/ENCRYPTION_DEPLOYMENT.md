# PII Field Encryption Deployment Guide

## Phase 1: Initial Deployment ✅ (Current)

**Status**: Implemented and ready for deployment
**Date**: 2026-02-09

### What Was Implemented

1. **Encryption Service** (`src/shared/encryption.rs`)
   - AES-256-GCM authenticated encryption
   - HMAC-SHA256 blind indexing for searchable fields
   - Random 12-byte nonces per encryption
   - Comprehensive unit tests

2. **Database Schema Changes**
   - Migration 1: Added `email_encrypted` and `email_index` to `expectations` table
   - Migration 2: Added `*_encrypted` and `*_index` columns to `contributors` table (4 fields)
   - Migration 3: Placeholder for data migration documentation

3. **Application Updates**
   - `ExpectationService`: Encrypts email field
   - `ContributorService`: Encrypts email, whatsapp, contact_email, contact_whatsapp
   - All services updated to write to `*_encrypted` columns
   - Response DTOs decrypt data for API clients

4. **Configuration**
   - Added `ENCRYPTION_KEY` environment variable (32-byte base64)
   - Added `ENCRYPTION_HMAC_KEY` environment variable (32-byte base64)
   - Updated `.env.example` with documentation

### Current Database Structure

Both plaintext and encrypted columns exist side-by-side:

**Expectations Table:**
- `email` (VARCHAR) - OLD plaintext column (remains NULL)
- `email_encrypted` (TEXT) - NEW encrypted column
- `email_index` (VARCHAR(64)) - Blind index for searching

**Contributors Table:**
- `email`, `whatsapp`, `contact_email`, `contact_whatsapp` - OLD plaintext columns (remain NULL)
- `email_encrypted`, `whatsapp_encrypted`, etc. - NEW encrypted columns
- `*_index` columns - Blind indexes

### Deployment Checklist

- [x] Generate encryption keys
- [x] Add keys to `.env`
- [x] Run migrations (1-3 only)
- [x] Test encryption on local
- [x] Verify data is encrypted in database
- [x] Verify API responses return decrypted data

### Next Deployment Steps

**Before deploying to production:**

1. **Generate Production Keys**
   ```bash
   openssl rand -base64 32  # For ENCRYPTION_KEY
   openssl rand -base64 32  # For ENCRYPTION_HMAC_KEY
   ```

2. **Store Keys Securely**
   - Add to secrets manager (AWS Secrets Manager, HashiCorp Vault, etc.)
   - **NEVER commit keys to git**
   - Keep encrypted backup of keys (losing keys = permanent data loss)

3. **Deploy Application**
   - Migrations 1-3 will run automatically
   - Application will start writing to encrypted columns
   - Old plaintext columns remain empty

4. **Verify Deployment**
   - Check logs for "Encryption service initialized"
   - Test creating expectations and contributors
   - Query database to confirm encrypted storage

---

## Phase 2: Data Migration (Future - Manual)

**Status**: Not started
**Estimated Time**: TBD (depends on data volume)

### Prerequisites

- Phase 1 deployed and stable for at least 1-2 weeks
- All new data is being encrypted successfully
- Database backup created and verified

### Steps

1. **Backup Database**
   ```bash
   pg_dump $DATABASE_URL > backup_before_migration.sql
   ```

2. **Run Migration Script**
   ```bash
   cargo run --bin migrate_pii_encryption
   ```

3. **Verify Migration**
   - Check script output for errors
   - Verify roundtrip encryption/decryption
   - Spot-check database records

4. **Monitor**
   - Watch for errors in application logs
   - Monitor database performance
   - Verify API responses

### Data Migration Script

Location: `src/bin/migrate_pii_encryption.rs`

**What it does:**
- Loads all records with plaintext PII
- Encrypts each field using EncryptionService
- Generates blind indexes
- Updates `*_encrypted` and `*_index` columns
- Verifies roundtrip decryption for data integrity

**Note**: The script currently has compilation issues that need to be fixed before use.

---

## Phase 3: Cleanup ✅ (Ready for Deployment)

**Status**: Implemented and ready for deployment
**Date**: 2026-02-09
**Timeline**: After Phase 2 + 1-2 week soak period

### Prerequisites

- Phase 2 completed successfully
- All existing data migrated and verified
- Application stable with encrypted data
- Soak period completed (1-2 weeks monitoring)

### What Was Implemented

1. **Database Migration** (`migrations/20260209175654_cleanup_pii_encryption.sql`)
   - Drops plaintext PII columns from `expectations` and `contributors` tables
   - Renames `*_encrypted` columns to original column names
   - Updates column comments to document encryption format
   - Includes safety checks with `DROP COLUMN IF EXISTS`

2. **Application Code Updates**
   - `ExpectationService`: Query updated to use `email` column (no longer `email_encrypted`)
   - `ContributorService`: All 4 PII field queries updated to use original column names
   - Removed `AS` aliases from `RETURNING` clauses

### Migration File

**File**: `migrations/20260209175654_cleanup_pii_encryption.sql`

This migration will run automatically with `sqlx migrate run` or on application startup.

### Steps

1. **Verify Prerequisites**

   Before running this migration, ensure:

   - ✅ Phase 2 data migration completed successfully
   - ✅ All plaintext columns are NULL (verified with database queries)
   - ✅ Application has been stable with encrypted data for 1-2 weeks
   - ✅ Database backup taken

2. **Take Database Backup**

   ```bash
   # CRITICAL: Take a full database backup before proceeding
   pg_dump $DATABASE_URL > backup_before_phase3_cleanup.sql

   # Verify backup was created successfully
   ls -lh backup_before_phase3_cleanup.sql
   ```

3. **Deploy Application with Migration**

   The migration will run automatically when the application starts (via `sqlx migrate run`).

   **For Production Deployment:**
   ```bash
   # Deploy using deployment script
   ./scripts/deploy.sh production
   ```

   **For Docker Deployments:**
   ```bash
   # Pull latest code
   git pull origin main

   # Rebuild and restart containers
   docker-compose down
   docker-compose up -d --build

   # Migration runs automatically on startup
   ```

4. **Verify Migration Success**

   ```bash
   # Check application logs
   docker logs <container-name> | grep migration

   # Should see: "Applied migration: 20260209175654_cleanup_pii_encryption"
   ```

5. **Verify Database Schema**

   ```sql
   -- Check expectations table
   SELECT column_name, data_type
   FROM information_schema.columns
   WHERE table_name = 'expectations'
   AND column_name LIKE '%email%';

   -- Should only see: email (TEXT), email_index (VARCHAR)
   -- Should NOT see: email_encrypted

   -- Check contributors table
   SELECT column_name, data_type
   FROM information_schema.columns
   WHERE table_name = 'contributors'
   AND column_name IN ('email', 'whatsapp', 'contact_email', 'contact_whatsapp');

   -- All should be TEXT (encrypted columns)
   ```

6. **Test Application**

   ```bash
   # Create a test expectation
   curl -X POST http://your-api/api/expectations \
     -H "Content-Type: application/json" \
     -d '{"name": "Test User", "email": "test@example.com", "expectation": "Test"}'

   # Create a test contributor
   curl -X POST http://your-api/api/contributors/register \
     -H "Content-Type: application/json" \
     -d '{"submission_type": "individual", "name": "Test", "email": "test@example.com", ...}'

   # Verify responses include decrypted data
   ```

7. **Update SQLx Offline Metadata** (for CI/CD)

   After successful deployment, update the offline query metadata:
   ```bash
   cargo sqlx prepare
   git add .sqlx/
   git commit -m "chore: update sqlx metadata after Phase 3 migration"
   git push
   ```

### Rollback Plan

**IMPORTANT**: This migration is **destructive** - it drops plaintext columns permanently.

**If issues arise after deployment:**

1. **Restore from backup** (taken in step 2)
   ```bash
   # Stop application
   docker-compose down

   # Restore database
   psql $DATABASE_URL < backup_before_phase3_cleanup.sql

   # Revert to previous application version
   git revert HEAD
   ./scripts/deploy.sh production
   ```

2. **The plaintext columns will be restored** with NULL values
3. **Encrypted columns will be restored** with their names: `*_encrypted`
4. **Application will continue working** with Phase 1/2 code

**Prevention**: Only run this migration after confirming Phase 2 is stable for 1-2 weeks.

### Post-Deployment Checklist

- [ ] Database backup verified before deployment
- [ ] Migration ran successfully (check logs)
- [ ] Plaintext columns dropped (verified via database query)
- [ ] Encrypted columns renamed to original names
- [ ] Application started without errors
- [ ] Create/read operations work correctly
- [ ] API responses contain decrypted data
- [ ] No PII in application logs
- [ ] SQLx offline metadata updated

### Final State After Phase 3

**Database Schema:**
- `expectations.email` - AES-256-GCM encrypted (TEXT)
- `expectations.email_index` - HMAC blind index (VARCHAR 64)
- `contributors.email`, `whatsapp`, `contact_email`, `contact_whatsapp` - All encrypted (TEXT)
- Corresponding `*_index` columns for blind indexes

**Application Behavior:**
- All PII fields encrypted before INSERT
- All PII fields decrypted for API responses
- Blind indexes used for equality searches
- No plaintext PII stored in database
- No PII in application logs

**Migration History:**
1. ✅ Phase 1: Added encrypted columns alongside plaintext
2. ✅ Phase 2: Migrated existing data to encrypted columns
3. ✅ Phase 3: Dropped plaintext columns and renamed encrypted columns

---

## Issues & TODO

### Known Issues

1. **Migration Binary Compilation Error**
   - File: `src/bin/migrate_pii_encryption.rs`
   - Issue: Type conversion errors with `String` vs `&str`
   - Impact: Cannot run data migration script yet
   - **TODO**: Fix compilation errors before Phase 2

2. **Deprecated Nonce::from_slice**
   - File: `src/shared/encryption.rs:87`
   - Warning: Using deprecated `GenericArray::from_slice`
   - Impact: Works but generates compiler warnings
   - **TODO**: Update to use newer API

3. **Dead Code Warnings**
   - File: `src/features/expectations/models/expectation.rs:14`
   - Warning: Field `email_index` never read
   - Impact: None (false positive, field is used in queries)
   - **TODO**: Can ignore or suppress warning

### Post-Deployment Cleanup Tasks

#### Immediate (After Phase 1 Deploy)

- [ ] Monitor application logs for encryption errors
- [ ] Verify all new records are being encrypted
- [ ] Check database to ensure plaintext columns stay NULL
- [ ] Test API endpoints thoroughly
- [ ] Monitor performance impact

#### Before Phase 2

- [ ] Fix `migrate_pii_encryption.rs` compilation errors
- [ ] Test migration script on staging/dev environment
- [ ] Document data volume and estimated migration time
- [ ] Plan maintenance window for migration

#### Before Phase 3

- [ ] Verify all data successfully migrated
- [ ] Confirm 1-2 week soak period completed
- [ ] Create manual cleanup migration file
- [ ] Update application code to remove aliases
- [ ] Test cleanup migration on staging

### Security Considerations

#### Key Management

- [ ] Move encryption keys to secrets manager (production)
- [ ] Document key rotation procedure
- [ ] Create encrypted backup of keys
- [ ] Limit access to keys (need-to-know basis)

#### Monitoring

- [ ] Set up alerts for encryption/decryption errors
- [ ] Monitor for PII leakage in logs
- [ ] Regular security audits of encrypted data

#### Compliance

- [ ] Document encryption implementation for compliance
- [ ] Verify blind index doesn't leak sensitive info
- [ ] Review data access patterns

---

## Rollback Plan

### Phase 1 Rollback

If issues found after Phase 1 deployment:

1. **Application Level**
   - Revert to previous git commit
   - Redeploy previous version
   - Old code still works (plaintext columns exist but empty)

2. **Database Level**
   - No rollback needed (plaintext columns remain)
   - Can optionally drop `*_encrypted` columns if needed

### Phase 2 Rollback

If migration fails:

1. Restore from backup:
   ```bash
   psql $DATABASE_URL < backup_before_migration.sql
   ```

2. Fix issues and retry migration

### Phase 3 Rollback

**WARNING**: Phase 3 is destructive! After running cleanup migration:

- Cannot rollback easily (plaintext columns deleted)
- Must restore from backup if issues found
- **Always test thoroughly before running Phase 3**

---

## Testing Checklist

### Local Testing ✅

- [x] Encryption service unit tests pass
- [x] Create expectation with email (encrypted)
- [x] Create contributor with PII fields (encrypted)
- [x] Verify database shows encrypted data
- [x] Verify API returns decrypted data
- [x] Verify blind indexes are deterministic

### Staging Testing (TODO)

- [ ] Deploy to staging environment
- [ ] Generate staging encryption keys
- [ ] Run migrations
- [ ] Test all PII-related endpoints
- [ ] Verify encryption with real-like data
- [ ] Performance testing with large datasets
- [ ] Test migration script (if data exists)

### Production Testing (TODO)

- [ ] Smoke tests after deployment
- [ ] Verify encryption in production database
- [ ] Monitor error rates
- [ ] Monitor API response times
- [ ] Verify logs show no PII

---

## Contact & Support

For issues or questions about this deployment:

- **Documentation**: This file
- **Implementation Plan**: See git history for detailed plan
- **Migration Script**: `src/bin/migrate_pii_encryption.rs`
- **Tests**: `src/shared/encryption.rs` (unit tests at bottom)

---

## Change Log

### 2026-02-09 - Phase 1 Implementation
- Implemented AES-256-GCM encryption service
- Added database migrations (1-3)
- Updated ExpectationService and ContributorService
- Generated encryption keys
- Tested locally - all working ✅
- **Status**: Ready for deployment
