//! Storage module for file management
//!
//! Provides MinIO/S3-compatible storage client for file uploads,
//! downloads, and presigned URL generation.

mod minio_client;

pub use minio_client::{FileVisibility, MinIOClient};
