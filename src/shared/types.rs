use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub meta: Option<Meta>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Meta {
    pub total: i64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: Option<T>, message: Option<String>, meta: Option<Meta>) -> Self {
        Self {
            success: true,
            data,
            message,
            meta,
            errors: None,
        }
    }

    pub fn error(message: Option<String>, errors: Option<Vec<String>>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message,
            meta: None,
            errors,
        }
    }
}
