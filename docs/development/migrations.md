# Database Migrations Guide

This project uses **SQLx built-in migrations** for database schema management.

## Overview

- **Migration tool:** SQLx CLI (`sqlx-cli`)
- **Format:** Flat SQL files in `migrations/` directory
- **Tracking:** `_sqlx_migrations` table
- **Auto-run:** Migrations run automatically on app startup

---

## Prerequisites

### Install SQLx CLI

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

### Database Connection

Set the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/db_your_app"
```

Or add to `.env`:
```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/db_your_app
```

---

## Common Tasks

### 1. Create a New Migration

```bash
sqlx migrate add <migration_name>
```

**Example:**
```bash
sqlx migrate add create_users_table
```

This creates: `migrations/<timestamp>_create_users_table.sql`

**Edit the file:**
```sql
-- migrations/20250102120000_create_users_table.sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

---

### 2. Run Pending Migrations

#### Development

Migrations run **automatically** when you start the app:

```bash
cargo run
```

Or run manually:

```bash
sqlx migrate run
```

#### Production

Migrations also run automatically on app startup. If you prefer manual control, you can disable auto-migration in code and run:

```bash
sqlx migrate run --database-url $DATABASE_URL
```

---

### 3. Check Migration Status

```bash
sqlx migrate info
```

**Output:**
```
20250101000000/applied create users table
20250102120000/pending create products table
```

---

### 4. Rollback Migrations

SQLx **does not have built-in rollback**. To rollback, create a **revert migration**:

```bash
sqlx migrate add revert_create_users_table
```

**Edit the revert migration:**
```sql
-- migrations/20250102130000_revert_create_users_table.sql
DROP INDEX IF EXISTS idx_users_email;
DROP TABLE IF EXISTS users;
```

Then run:
```bash
sqlx migrate run
```

**Best Practice:** Name revert migrations clearly:
- `revert_<original_migration_name>`
- Or `rollback_<feature_name>`

---

## Migration File Naming

SQLx uses timestamp-based naming:

```
<timestamp>_<description>.sql
```

**Examples:**
- `20250101000000_create_users_table.sql`
- `20250102120000_add_user_roles.sql`
- `20250103120000_create_products_table.sql`

**Timestamp format:** `YYYYMMDDHHmmss`

---

## Best Practices

### 1. One Logical Change per Migration

✅ **Good:**
```sql
-- 20250102120000_add_users_table.sql
CREATE TABLE users (...);
CREATE INDEX idx_users_email ON users(email);
```

❌ **Bad:**
```sql
-- 20250102120000_multiple_changes.sql
CREATE TABLE users (...);
CREATE TABLE posts (...);
ALTER TABLE products ADD COLUMN user_id UUID;
```

### 2. Make Migrations Idempotent

Use `IF NOT EXISTS` / `IF EXISTS` to allow re-running:

```sql
CREATE TABLE IF NOT EXISTS users (...);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
```

### 3. Test Migrations Before Production

Always test on development/staging first:

```bash
# Test database
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/db_your_app_dev"
sqlx migrate run

# Verify schema
psql $DATABASE_URL -c "\d users"
```

### 4. Keep Migrations in Version Control

✅ Commit migrations to git:
```bash
git add migrations/
git commit -m "Add users table migration"
```

### 5. Document Complex Migrations

Add comments for context:

```sql
-- migrations/20250102120000_add_user_roles.sql
-- Add role-based access control (RBAC) support
-- Related to: https://github.com/org/repo/issues/123

CREATE TYPE user_role AS ENUM ('admin', 'member', 'viewer');

ALTER TABLE users ADD COLUMN role user_role NOT NULL DEFAULT 'viewer';
CREATE INDEX idx_users_role ON users(role);
```

---

## Workflow Examples

### Adding a New Feature

```bash
# 1. Create migration
sqlx migrate add add_comments_feature

# 2. Edit migration file
# migrations/20250102120000_add_comments_feature.sql
# CREATE TABLE comments (...);

# 3. Run migration (or just start app)
cargo run

# 4. Verify in database
psql $DATABASE_URL -c "\d comments"

# 5. Commit
git add migrations/
git commit -m "feat: add comments feature with migrations"
```

---

### Rolling Back a Feature

```bash
# 1. Create revert migration
sqlx migrate add revert_comments_feature

# 2. Edit revert migration
# migrations/20250102130000_revert_comments_feature.sql
# DROP TABLE IF EXISTS comments;

# 3. Run migration
sqlx migrate run

# 4. Commit
git add migrations/
git commit -m "revert: remove comments feature"
```

---

### Renaming a Column (Zero-Downtime)

For production systems with zero downtime:

```bash
# Step 1: Add new column
sqlx migrate add add_full_name_column
```

```sql
-- migrations/20250102120000_add_full_name_column.sql
ALTER TABLE users ADD COLUMN full_name VARCHAR(255);
```

```bash
# Step 2: Deploy code that writes to both columns

# Step 3: Backfill data
sqlx migrate add backfill_full_name
```

```sql
-- migrations/20250102121000_backfill_full_name.sql
UPDATE users SET full_name = name WHERE full_name IS NULL;
```

```bash
# Step 4: Make new column NOT NULL
sqlx migrate add make_full_name_required
```

```sql
-- migrations/20250102122000_make_full_name_required.sql
ALTER TABLE users ALTER COLUMN full_name SET NOT NULL;
```

```bash
# Step 5: Drop old column
sqlx migrate add drop_name_column
```

```sql
-- migrations/20250102123000_drop_name_column.sql
ALTER TABLE users DROP COLUMN name;
```

---

## Troubleshooting

### "Migration checksum mismatch"

**Cause:** Migration file was modified after being applied.

**Solution:**
1. **Never modify applied migrations** in production
2. For development, reset database:
   ```bash
   dropdb db_your_app_dev
   createdb db_your_app_dev
   sqlx migrate run
   ```

### "Database is locked"

**Cause:** Long-running migration holding locks.

**Solution:**
1. Check running queries:
   ```sql
   SELECT * FROM pg_stat_activity WHERE datname = 'db_your_app';
   ```
2. Kill blocking queries if safe:
   ```sql
   SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = 'db_your_app';
   ```

### "Permission denied"

**Cause:** Database user lacks required permissions.

**Solution:**
```sql
ALTER USER postgres CREATEDB;
GRANT ALL PRIVILEGES ON DATABASE db_your_app TO postgres;
```

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Test & Deploy

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install sqlx-cli
        run: cargo install sqlx-cli --no-default-features --features postgres

      - name: Run migrations
        run: sqlx migrate run
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/db_your_app

      - name: Run tests
        run: cargo test
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/db_your_app
```

---

## Comparison: Old vs New System

| Feature | Old (Custom Migrator) | New (SQLx Built-in) |
|---------|----------------------|---------------------|
| **Format** | Folder-based (`up.sql`/`down.sql`) | Flat files (`*.sql`) |
| **Commands** | `cargo run -- migrate-up` | `sqlx migrate run` |
| **Rollback** | Built-in `down.sql` | Manual revert migrations |
| **Tracking table** | `_migrations` | `_sqlx_migrations` |
| **Auto-run** | ✅ Yes | ✅ Yes |
| **Industry standard** | ❌ Custom | ✅ Yes |
| **Tooling** | Custom CLI | `sqlx-cli` |
| **Test integration** | Manual | `#[sqlx::test]` macro |

---

## Additional Resources

- [SQLx Documentation](https://docs.rs/sqlx/latest/sqlx/)
- [SQLx Migrations Guide](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#create-and-run-migrations)
- [PostgreSQL Best Practices](https://wiki.postgresql.org/wiki/Don%27t_Do_This)

---

**Questions or issues?** Check the project's GitHub issues or contact the team.
