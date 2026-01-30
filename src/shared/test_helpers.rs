#[cfg(test)]
use crate::features::auth::model::AuthenticatedUser;

#[cfg(test)]
use axum::{extract::Request, middleware::Next, response::Response, Router};

#[cfg(test)]
#[allow(dead_code)]
pub fn create_super_admin_user() -> AuthenticatedUser {
    AuthenticatedUser {
        account_id: "test-account-id".to_string(),
        sub: "test-sub".to_string(),
        session_uid: Some("test-session-uid".to_string()),
        roles: vec!["super_admin".to_string()],
    }
}

#[cfg(test)]
#[allow(dead_code)]
async fn inject_super_admin_middleware(mut request: Request, next: Next) -> Response {
    request.extensions_mut().insert(create_super_admin_user());
    next.run(request).await
}

#[cfg(test)]
#[allow(dead_code)]
pub fn with_super_admin_auth(router: Router) -> Router {
    router.layer(axum::middleware::from_fn(inject_super_admin_middleware))
}
