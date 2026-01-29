use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, post},
    Router,
};
use std::sync::Arc;

use crate::features::files::dtos::MAX_FILE_SIZE;
use crate::features::files::handlers::{delete_file_by_url, upload_file};
use crate::features::files::services::FileService;

/// Create routes for the files feature
pub fn routes(file_service: Arc<FileService>) -> Router {
    Router::new()
        .route(
            "/api/files/upload",
            // Allow body size up to MAX_FILE_SIZE + buffer for multipart overhead
            post(upload_file).layer(DefaultBodyLimit::max(MAX_FILE_SIZE + 1024 * 1024)),
        )
        .route("/api/files", delete(delete_file_by_url))
        .with_state(file_service)
}
