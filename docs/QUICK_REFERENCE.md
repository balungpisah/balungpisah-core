# Quick Reference

## Database Migrations

```bash
# Create new migration
sqlx migrate add <name>

# Run migrations (or just start the app)
sqlx migrate run

# Check status
sqlx migrate info

# Rollback (create revert migration)
sqlx migrate add revert_<name>
```

See [MIGRATIONS.md](./MIGRATIONS.md) for detailed guide.

---

## Testing

```bash
# Run all tests (parallel, fast)
cargo test

# Run specific test
cargo test test_create_demo

# Run with output
cargo test -- --nocapture

# Run specific module
cargo test service_test
cargo test handler_test
```

See [TESTING.md](./TESTING.md) for detailed guide.

---

## Development

```bash
# Start dev server
cargo run

# Check code
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy

# Watch mode (requires cargo-watch)
cargo watch -x run
```

---

## Environment Setup

```bash
# Required
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/balungpisah"

# For tests
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/balungpisah_test"

# Or create .env file
cat > .env << EOF
DATABASE_URL=postgres://postgres:postgres@localhost:5432/balungpisah
EOF
```

---

## Common Workflows

### Adding New Feature with Migration

```bash
# 1. Create migration
sqlx migrate add add_users_table

# 2. Edit migrations/<timestamp>_add_users_table.sql
# Write your SQL...

# 3. Run migration (or start app)
cargo run

# 4. Write code & tests
# ...

# 5. Run tests
cargo test

# 6. Commit
git add .
git commit -m "feat: add users table"
```

### Rolling Back a Migration

```bash
# 1. Create revert migration
sqlx migrate add revert_users_table

# 2. Edit migrations/<timestamp>_revert_users_table.sql
# Write DROP/ALTER statements...

# 3. Run migration
sqlx migrate run

# 4. Commit
git add .
git commit -m "revert: remove users table"
```

---

## Troubleshooting

### Tests fail with "relation not exists"
```bash
# Restart PostgreSQL to clear orphan test DBs
docker restart shared-postgres

# Create base test DB
createdb balungpisah_test

# Run tests again
cargo test
```

### "Too many clients already"
```bash
# Restart PostgreSQL
docker restart shared-postgres

# Wait a few seconds
sleep 5

# Try again
cargo test
```

### Migration checksum mismatch
```bash
# Development: Reset database
dropdb balungpisah_dev
createdb balungpisah_dev
sqlx migrate run
```

---

## Project Structure

```
balungpisah-core/
├── migrations/              # Database migrations (SQLx format)
├── src/
│   ├── core/               # Core utilities
│   │   ├── config.rs
│   │   ├── database.rs
│   │   ├── error.rs
│   │   └── middleware.rs
│   ├── features/           # Feature modules (plural naming)
│   │   ├── auth/           # Authentication & authorization
│   │   └── <feature>/      # Each feature follows same structure
│   │       ├── dtos/
│   │       │   ├── <name>_dto.rs
│   │       │   └── mod.rs
│   │       ├── models/
│   │       │   ├── <name>_model.rs
│   │       │   └── mod.rs
│   │       ├── services/
│   │       │   ├── <name>_service.rs
│   │       │   ├── <name>_service_test.rs
│   │       │   └── mod.rs
│   │       ├── handlers/
│   │       │   ├── <name>_handler.rs
│   │       │   ├── <name>_handler_test.rs
│   │       │   └── mod.rs
│   │       ├── routes.rs
│   │       ├── test_utils.rs
│   │       └── mod.rs
│   ├── shared/             # Shared types
│   └── main.rs
├── docs/                   # Documentation
└── README.md
```

---

## Links

- [Migrations Guide](./MIGRATIONS.md)
- [Testing Guide](./TESTING.md)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Axum Documentation](https://docs.rs/axum/)
