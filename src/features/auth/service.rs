use crate::core::error::Result;
use crate::features::auth::dto::MeResponseDto;
use crate::features::auth::model::AuthenticatedUser;

pub struct AuthService;

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_current_user(&self, user: AuthenticatedUser) -> Result<MeResponseDto> {
        Ok(user.into())
    }
}
