//! Contributor registration feature for public contributor sign-ups.
//!
//! This feature provides a public endpoint for contributor registration
//! that stores form data directly without authentication.
//!
//! ## Endpoints
//!
//! | Method | Endpoint | Auth | Description |
//! |--------|----------|------|-------------|
//! | POST | `/api/contributors/register` | No | Register new contributor |

pub mod dtos;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

pub use services::ContributorService;
