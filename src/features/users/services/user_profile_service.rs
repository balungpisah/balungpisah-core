use crate::core::error::Result;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::users::clients::logto::users::{
    LogtoUserProfileClient, LogtoUserProfileUpdate, UpdateUserProfileRequest, UpdateUserRequest,
};
use crate::features::users::dtos::{
    ExtendedProfileDto, UpdateBasicProfileDto, UpdateExtendedProfileDto, UserProfileResponseDto,
};
use std::sync::Arc;

/// Service for managing user profiles via Logto Management API
pub struct UserProfileService {
    users_client: Arc<LogtoUserProfileClient>,
}

impl UserProfileService {
    pub fn new(users_client: Arc<LogtoUserProfileClient>) -> Self {
        Self { users_client }
    }

    /// Get current user's profile with roles from JWT
    pub async fn get_profile(&self, user: &AuthenticatedUser) -> Result<UserProfileResponseDto> {
        // Fetch user from Logto API
        let logto_user = self.users_client.get_user(&user.sub).await?;

        // Convert to response DTO and enrich with JWT context
        let mut response: UserProfileResponseDto = logto_user.into();
        response.roles = user.roles.clone();

        Ok(response)
    }

    /// Update basic profile fields (name, avatar, username)
    pub async fn update_basic_profile(
        &self,
        user: &AuthenticatedUser,
        dto: UpdateBasicProfileDto,
    ) -> Result<UserProfileResponseDto> {
        let request = UpdateUserRequest {
            name: dto.name,
            avatar: dto.avatar,
            username: dto.username,
        };

        let updated_user = self.users_client.update_user(&user.sub, request).await?;

        // Convert to response DTO and enrich with JWT context
        let mut response: UserProfileResponseDto = updated_user.into();
        response.roles = user.roles.clone();

        Ok(response)
    }

    /// Update extended profile fields (OIDC standard claims)
    pub async fn update_extended_profile(
        &self,
        user: &AuthenticatedUser,
        dto: UpdateExtendedProfileDto,
    ) -> Result<ExtendedProfileDto> {
        let request = UpdateUserProfileRequest {
            profile: LogtoUserProfileUpdate {
                given_name: dto.given_name,
                family_name: dto.family_name,
                nickname: dto.nickname,
                birthdate: dto.birthdate,
                locale: dto.locale,
                zoneinfo: dto.zoneinfo,
                gender: dto.gender,
                website: dto.website,
            },
        };

        let updated_profile = self
            .users_client
            .update_user_profile(&user.sub, request)
            .await?;

        Ok(updated_profile.into())
    }
}
