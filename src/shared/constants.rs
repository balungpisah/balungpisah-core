/// Default page size for pagination
#[allow(dead_code)]
pub const DEFAULT_PAGE_SIZE: i64 = 10;

/// Maximum page size allowed
#[allow(dead_code)]
pub const MAX_PAGE_SIZE: i64 = 100;

// =============================================================================
// ROLE CONSTANTS
// =============================================================================

/// Admin curator role - can curate citizen reports and manage marketplace items
#[allow(dead_code)]
pub const ROLE_ADMIN_CURATOR: &str = "admin_curator";

/// Citizen role - can report problems and track their reports
#[allow(dead_code)]
pub const ROLE_CITIZEN: &str = "citizen";

/// Official role - can claim and resolve marketplace problems
#[allow(dead_code)]
pub const ROLE_OFFICIAL: &str = "official";
