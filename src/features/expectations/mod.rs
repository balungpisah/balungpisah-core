//! User expectations feature for landing page submissions.
//!
//! This feature provides a public endpoint for collecting user expectations
//! from the "Coming Soon" landing page before the full platform launch.
//!
//! ## Endpoints
//!
//! | Method | Endpoint | Auth | Description |
//! |--------|----------|------|-------------|
//! | POST | `/api/expectations` | No | Submit user expectation |

pub mod dtos;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

pub use services::ExpectationService;
