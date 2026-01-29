//! User profile management feature.
//!
//! This feature provides endpoints for users to manage their personal information
//! by integrating with Logto Management API.
//!
//! ## Endpoints
//!
//! | Method | Endpoint | Description |
//! |--------|----------|-------------|
//! | GET | `/api/me` | Get profile + organization + roles |
//! | PATCH | `/api/me` | Update basic profile (name, avatar, username) |
//! | PATCH | `/api/me/profile` | Update extended profile (birthdate, locale, etc.) |

pub mod clients;
pub mod dtos;
pub mod handlers;
pub mod routes;
pub mod services;
