use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::shared::types::ApiResponse;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, errors) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error occurred".to_string(),
                    None,
                )
            }
            AppError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.clone(), None),
            AppError::Validation(ref msg) => (
                StatusCode::BAD_REQUEST,
                msg.clone(),
                Some(vec![msg.clone()]),
            ),
            AppError::BadRequest(ref msg) => (StatusCode::BAD_REQUEST, msg.clone(), None),
            AppError::Internal(ref msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                    None,
                )
            }
            AppError::Auth(ref msg) => (StatusCode::UNAUTHORIZED, msg.clone(), None),
            AppError::Unauthorized(ref msg) => (StatusCode::UNAUTHORIZED, msg.clone(), None),
            AppError::Forbidden(ref msg) => (StatusCode::FORBIDDEN, msg.clone(), None),
            AppError::Conflict(ref msg) => (StatusCode::CONFLICT, msg.clone(), None),
            AppError::ExternalServiceError(ref msg) => {
                tracing::error!("External service error: {}", msg);
                (StatusCode::BAD_GATEWAY, msg.clone(), None)
            }
            AppError::RateLimitExceeded(ref msg) => {
                (StatusCode::TOO_MANY_REQUESTS, msg.clone(), None)
            }
        };

        let body = Json(ApiResponse::<()>::error(Some(message), errors));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
