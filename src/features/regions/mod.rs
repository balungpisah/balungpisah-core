//! Indonesian administrative regions (wilayah) feature.
//!
//! This feature provides endpoints for accessing Indonesian administrative regions
//! data including provinces, regencies/cities, districts, and villages.
//!
//! ## Data Hierarchy
//!
//! - Level 1: Provinces (Provinsi) - 37 regions
//! - Level 2: Regencies/Cities (Kabupaten/Kota) - 514 regions
//! - Level 3: Districts (Kecamatan) - 7,257 regions
//! - Level 4: Villages (Kelurahan/Desa) - 82,830 regions
//!
//! ## Endpoints
//!
//! | Method | Endpoint | Description |
//! |--------|----------|-------------|
//! | GET | `/api/regions/provinces` | List all provinces |
//! | GET | `/api/regions/provinces/{code}` | Get province by code |
//! | GET | `/api/regions/provinces/{code}/regencies` | List regencies in a province |
//! | GET | `/api/regions/regencies/{code}` | Get regency by code |
//! | GET | `/api/regions/regencies/{code}/districts` | List districts in a regency |
//! | GET | `/api/regions/districts/{code}` | Get district by code |
//! | GET | `/api/regions/districts/{code}/villages` | List villages in a district |
//! | GET | `/api/regions/villages/{code}` | Get village by code |

pub mod dtos;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

pub use services::RegionService;
