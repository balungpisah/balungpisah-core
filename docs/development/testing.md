# Testing Guide

> **Philosophy:** Tests are not just for verification—they are documentation, safety nets, and design feedback.

## Core Principles

### 1. Isolation First
- **MUST:** Each test runs in its own isolated database
- **MUST:** Use `#[sqlx::test]` macro for all database-dependent tests
- **NEVER:** Share database state between tests
- **NEVER:** Use `--test-threads=1` or manual cleanup

### 2. Compile-Time Safety
- **MUST:** Prefer `sqlx::query!` and `sqlx::query_as!` (compile-time checked)
- **MUST:** Run `cargo sqlx prepare` after query changes
- **ONLY USE** runtime queries (`sqlx::query_as::<_, T>`) for:
  - Models with `#[sqlx(json)]` attribute
  - Dynamic `QueryBuilder` queries
- **MUST:** Document runtime queries with `// Runtime-checked due to...`

### 3. Multi-Tenancy Security
- **MUST:** Test owner isolation for all CRUD operations
- **MUST:** Return `NotFound` (not `Forbidden`) for wrong owner
- **NEVER:** Expose resource existence across tenants
- **MUST:** Test both read and write operations with different owners

### 4. Test Organization
- **Service tests:** Place in `services/<name>_service_test.rs`
- **Handler tests:** Place in `handlers/<name>_handler_test.rs`
- **Shared fixtures:** Use `test_utils.rs` at feature level
- **Complex fixtures:** Use `test_utils/` subfolder within services/handlers
- **MUST:** Include test modules in respective `mod.rs` with `#[cfg(test)]`
- **MUST:** Group fixtures by entity/service

---

## Test Structure Templates

### Standard Feature Structure

```
src/features/<feature>/
├── dtos/
│   ├── <name>_dto.rs
│   └── mod.rs
├── models/
│   ├── <name>_model.rs
│   └── mod.rs
├── services/
│   ├── <name>_service.rs
│   ├── <name>_service_test.rs   # Service tests
│   └── mod.rs
├── handlers/
│   ├── <name>_handler.rs
│   ├── <name>_handler_test.rs   # Handler tests
│   └── mod.rs
├── routes.rs
├── test_utils.rs                 # Shared test fixtures
└── mod.rs
```

### Complex Feature (Multiple Services)

```
src/features/<feature>/
├── services/
│   ├── service_a.rs
│   ├── service_a_test.rs
│   ├── service_b.rs
│   ├── service_b_test.rs
│   ├── test_utils/              # Subfolder for complex fixtures
│   │   ├── mod.rs
│   │   ├── service_a_fixtures.rs
│   │   └── service_b_fixtures.rs
│   └── mod.rs
├── handlers/
│   ├── <name>_handler.rs
│   ├── <name>_handler_test.rs
│   └── mod.rs
├── test_utils.rs                 # Feature-level shared fixtures
└── mod.rs
```

**Include in `services/mod.rs`:**
```rust
mod service_a;
mod service_b;
pub use service_a::*;
pub use service_b::*;

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod service_a_test;

#[cfg(test)]
mod service_b_test;
```

**Include in `handlers/mod.rs`:**
```rust
mod <name>_handler;
pub use <name>_handler::*;

#[cfg(test)]
mod <name>_handler_test;
```

---

## Test Utilities Pattern

### Seed Functions (Simple Cases)
For straightforward entity creation:

```rust
pub async fn seed_entity(pool: &PgPool, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query!("INSERT INTO entities (id, name) VALUES ($1, $2)", id, name)
        .execute(pool).await.expect("Failed to seed");
    id
}
```

### Builder Pattern (Complex Entities)
For entities with:
- Many optional fields
- Configuration dependencies (e.g., encryption)
- Complex relationships

```rust
pub struct EntityBuilder {
    // Required fields in constructor
    // Optional fields as Option<T>
}

impl EntityBuilder {
    pub fn new(required_field: String) -> Self { /* defaults */ }
    pub fn optional_field(mut self, value: T) -> Self { /* builder */ }
    pub async fn create(self, pool: &PgPool) -> Uuid { /* creation */ }
}
```

**When to use:**
- Entity has 3+ optional fields
- Needs service dependencies (e.g., encryption config)
- Tests need flexible data variations

### Batch Creation
For pagination and performance tests:

```rust
pub async fn seed_batch(pool: &PgPool, count: usize) -> Vec<Uuid> {
    (1..=count).map(|i| seed_entity(pool, &format!("Entity {}", i)).await).collect()
}
```

---

## Critical Test Cases

### ✅ MUST Test: Owner Isolation

```rust
#[sqlx::test]
async fn test_get_wrong_owner(pool: sqlx::PgPool) {
    seed_entity(&pool, "owner_1", "Entity 1").await;
    let result = service.get("owner_2", entity_id).await;
    assert!(matches!(result, Err(AppError::NotFound(_))));
}
```

**Cover:** GET, UPDATE, DELETE for all entities

### ✅ MUST Test: Data Encryption

For any encrypted fields:

```rust
#[sqlx::test]
async fn test_field_encrypted_in_db(pool: sqlx::PgPool) {
    let entity_id = create_with_sensitive_data(&pool).await;
    let raw = sqlx::query!("SELECT sensitive_field FROM table WHERE id = $1", entity_id)
        .fetch_one(&pool).await.unwrap();
    assert_ne!(raw.sensitive_field, "plaintext");
}
```

**Also test:** Decryption roundtrip via service methods

### ✅ MUST Test: Uniqueness Constraints

For fields with uniqueness (slugs, emails, etc.):

```rust
#[sqlx::test]
async fn test_unique_field_conflict_resolution(pool: sqlx::PgPool) {
    let first = create(&pool, "Name").await;   // slug: "name"
    let second = create(&pool, "Name").await;  // slug: "name-2"
    assert_ne!(first.slug, second.slug);
}
```

### ✅ MUST Test: Cascade Deletes

For parent-child relationships:

```rust
#[sqlx::test]
async fn test_delete_parent_cascades(pool: sqlx::PgPool) {
    let (parent_id, child_id) = seed_with_children(&pool).await;
    service.delete(parent_id).await.unwrap();

    // Verify child is deleted
    let child_exists = sqlx::query!("SELECT id FROM children WHERE id = $1", child_id)
        .fetch_optional(&pool).await.unwrap();
    assert!(child_exists.is_none());
}
```

### ✅ MUST Test: Pagination Edge Cases

```rust
#[sqlx::test]
async fn test_pagination_boundaries(pool: sqlx::PgPool) {
    seed_batch(&pool, 25).await;

    // First page: 10 items
    // Last page: 5 items
    // Beyond last: 0 items
    // Total always correct
}
```

---

## Test Naming Convention

**Pattern:** `test_<operation>_<scenario>`

**Examples:**
- ✅ `test_create_success`
- ✅ `test_get_by_id_not_found`
- ✅ `test_update_wrong_owner`
- ✅ `test_list_with_pagination`
- ❌ `test_1` (not descriptive)
- ❌ `test_update` (no scenario)

---

## What NOT to Test

### ❌ DON'T Test Framework Behavior
```rust
// Bad: Testing SQLx itself
#[sqlx::test]
async fn test_database_connection_works(pool: sqlx::PgPool) {
    assert!(pool.acquire().await.is_ok());
}
```

### ❌ DON'T Test DTOs/Models Directly
Validation is tested via service layer integration

### ❌ DON'T Duplicate Tests
If service A depends on service B, test B's behavior in B's tests only

---

## Service Dependencies Pattern

When a service uses another service internally:

```rust
// Good: Test integration, not internals
#[sqlx::test]
async fn test_parent_service_uses_child_correctly(pool: sqlx::PgPool) {
    let config = create_test_config();
    let parent_service = ParentService::new(pool.clone(), config);

    let result = parent_service.create(dto).await.unwrap();

    // Assert on outcomes, not child service internals
    assert_eq!(result.processed_field, expected_value);
}
```

**DON'T:** Mock child services—use real database instead
**DO:** Test error propagation from child to parent

---

## Running Tests

### Development Workflow

```bash
# Run all tests
cargo test

# Run specific feature tests
cargo test products::services

# Run specific test file
cargo test product_service_test

# Run single test with output
cargo test test_create -- --nocapture

# After query changes
cargo sqlx prepare
```

### CI/CD Requirements

```yaml
steps:
  - name: Run tests
    run: cargo test
    env:
      DATABASE_URL: postgres://user:pass@localhost/test_db
```

**MUST:**
- PostgreSQL service running
- Base database created (macro creates test DBs from it)
- User has `CREATEDB` permission

---

## Troubleshooting

### "database does not exist"
**Cause:** Base database not created
**Fix:** `createdb <base_db_name>`

### "relation does not exist"
**Cause:** Migrations not run
**Fix:** Ensure `./migrations/` exists and `DATABASE_URL` is correct

### "unexpected null" in queries
**Cause:** Using runtime query without handling NULL
**Fix:** Use compile-time checked query or handle Option<T>

### Slow tests
**Cause:** Too many database operations
**Fix:**
- Use batch inserts where possible
- Reduce test data volume
- Check for N+1 queries

---

## Arrange-Act-Assert Pattern

**ALWAYS follow this structure:**

```rust
#[sqlx::test]
async fn test_example(pool: sqlx::PgPool) {
    // Arrange: Set up test data
    let entity_id = seed_entity(&pool).await;
    let service = Service::new(pool);

    // Act: Execute the operation
    let result = service.operation(entity_id).await;

    // Assert: Verify outcomes
    assert!(result.is_ok());
    assert_eq!(result.unwrap().field, expected);
}
```

**Benefits:**
- Clear test intent
- Easy to debug failures
- Consistent structure across codebase

---

## Coverage Checklist

For each service method, ensure tests cover:

- [ ] ✅ Happy path (success case)
- [ ] ❌ Not found error
- [ ] 👥 Wrong owner (if multi-tenant)
- [ ] ✏️ Validation errors (if applicable)
- [ ] 🔍 Edge cases (empty results, boundaries)
- [ ] 🔗 Related data (cascades, relationships)

**Example:** For a `list` method:
- Empty list
- List with data
- Pagination (first, middle, last, beyond)
- Filtering combinations
- Search functionality

---

## Migration Testing

**Tests automatically use production migrations from `./migrations/`**

**Benefits:**
- Schema changes tested automatically
- No drift between test and production
- Migration failures caught early

**After migration changes:**
```bash
cargo sqlx prepare  # Update metadata
cargo test          # Verify tests pass
```

---

## Quick Reference

### Test File Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::AppError;

    #[sqlx::test]
    async fn test_operation(pool: sqlx::PgPool) {
        // Arrange
        // Act
        // Assert
    }
}
```

### Common Assertions
```rust
// Success
assert!(result.is_ok());

// Error type
assert!(matches!(result, Err(AppError::NotFound(_))));

// Values
assert_eq!(actual, expected);
assert_ne!(encrypted, plaintext);

// Collections
assert_eq!(items.len(), 10);
assert!(items.iter().all(|x| x.is_active));
```

---

**Remember:** Good tests are:
- **Fast** - Use minimal test data
- **Isolated** - No shared state
- **Readable** - Clear intent and structure
- **Comprehensive** - Cover happy and error paths
- **Maintainable** - Follow consistent patterns
