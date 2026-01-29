use axum::{
    body::Body,
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;

use crate::core::error::AppError;
use crate::features::auth::model::AuthenticatedUser;

/// Custom JSON extractor that provides consistent error responses
pub struct AppJson<T>(pub T);

impl<T, S> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppJsonRejection;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => Err(AppJsonRejection(rejection)),
        }
    }
}

pub struct AppJsonRejection(JsonRejection);

impl IntoResponse for AppJsonRejection {
    fn into_response(self) -> Response {
        let message = match self.0 {
            JsonRejection::JsonDataError(err) => format!("Invalid JSON data: {}", err),
            JsonRejection::JsonSyntaxError(err) => format!("Invalid JSON syntax: {}", err),
            JsonRejection::MissingJsonContentType(err) => {
                format!("Missing JSON content type: {}", err)
            }
            _ => "Failed to parse JSON body".to_string(),
        };

        AppError::BadRequest(message).into_response()
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))
    }
}
