use crate::core::error::AppError;
use crate::features::auth::JwtValidator;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::prelude::*;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::request_id::{MakeRequestId, RequestId};
use tracing::Span;
use uuid::Uuid;

/// Request ID generator using UUID v7 (time-ordered)
#[derive(Clone, Copy)]
pub struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &axum::http::Request<B>) -> Option<RequestId> {
        let id = Uuid::now_v7().to_string();
        Some(RequestId::new(HeaderValue::from_str(&id).unwrap()))
    }
}

/// Custom MakeSpan that includes request_id in the tracing span
#[derive(Clone, Debug)]
pub struct MakeSpanWithRequestId;

impl<B> tower_http::trace::MakeSpan<B> for MakeSpanWithRequestId {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> Span {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        tracing::info_span!(
            "request",
            method = %request.method(),
            uri = %request.uri(),
            request_id = %request_id,
        )
    }
}

pub fn cors_layer(allowed_origins: Vec<String>) -> CorsLayer {
    let cors = CorsLayer::new().allow_methods(Any).allow_headers(Any);

    // If origins list contains "*", allow any origin
    if allowed_origins.iter().any(|o| o == "*") {
        cors.allow_origin(Any)
    } else {
        // Parse origins into HeaderValue
        let origins: Vec<HeaderValue> = allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors.allow_origin(AllowOrigin::list(origins))
    }
}

pub fn basic_auth_middleware(
    valid_credentials: Arc<String>,
) -> impl Fn(
    Request,
    Next,
)
    -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Response>> + Send>>
       + Clone {
    move |req: Request, next: Next| {
        let credentials = valid_credentials.clone();
        Box::pin(async move {
            let auth_header = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok());

            if let Some(auth_header) = auth_header {
                if let Some(encoded) = auth_header.strip_prefix("Basic ") {
                    if let Ok(decoded) = BASE64_STANDARD.decode(encoded) {
                        if let Ok(creds) = String::from_utf8(decoded) {
                            if creds == *credentials {
                                return Ok(next.run(req).await);
                            }
                        }
                    }
                }
            }

            let response = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::WWW_AUTHENTICATE, "Basic realm=\"Swagger UI\"")
                .body(Body::from("Unauthorized"))
                .unwrap();

            Err(response)
        })
    }
}

pub async fn auth_middleware(
    State(validator): State<Arc<JwtValidator>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    // Validate Bearer format
    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized(
            "Invalid authorization header format".to_string(),
        ));
    }

    let token = &auth_header[7..]; // Skip "Bearer "

    // Validate token
    let user = validator.validate_token(token).await?;

    // Insert authenticated user into request extensions
    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}
