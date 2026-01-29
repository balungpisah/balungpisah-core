use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::features::auth;
use crate::features::contributors::{dtos as contributors_dtos, handlers as contributors_handlers};
use crate::features::expectations::{dtos as expectations_dtos, handlers as expectations_handlers};
use crate::features::files::{dtos as files_dtos, handlers as files_handlers};
use crate::features::regions::{dtos as regions_dtos, handlers as regions_handlers};
use crate::features::users::{dtos as users_dtos, handlers::profile_handler};
use crate::shared::types::{ApiResponse, Meta};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Auth
        auth::handler::get_me,
        // Users
        profile_handler::get_profile,
        profile_handler::update_basic_profile,
        profile_handler::update_extended_profile,
        // Regions
        regions_handlers::list_provinces,
        regions_handlers::get_province,
        regions_handlers::list_regencies_by_province,
        regions_handlers::search_regencies,
        regions_handlers::get_regency,
        regions_handlers::list_districts_by_regency,
        regions_handlers::search_districts,
        regions_handlers::get_district,
        regions_handlers::list_villages_by_district,
        regions_handlers::search_villages,
        regions_handlers::get_village,
        // Files
        files_handlers::upload_file,
        files_handlers::delete_file_by_url,
        // Expectations (public)
        expectations_handlers::create_expectation,
        // Contributors (public)
        contributors_handlers::register_contributor,
    ),
    components(
        schemas(
            // Shared
            Meta,
            // Auth
            auth::dto::MeResponseDto,
            auth::model::AuthenticatedUser,
            ApiResponse<auth::dto::MeResponseDto>,
            // Users
            users_dtos::UserProfileResponseDto,
            users_dtos::ExtendedProfileDto,
            users_dtos::UpdateBasicProfileDto,
            users_dtos::UpdateExtendedProfileDto,
            ApiResponse<users_dtos::UserProfileResponseDto>,
            ApiResponse<users_dtos::ExtendedProfileDto>,
            // Regions
            regions_dtos::ProvinceResponseDto,
            regions_dtos::RegencyResponseDto,
            regions_dtos::DistrictResponseDto,
            regions_dtos::VillageResponseDto,
            ApiResponse<Vec<regions_dtos::ProvinceResponseDto>>,
            ApiResponse<regions_dtos::ProvinceResponseDto>,
            ApiResponse<Vec<regions_dtos::RegencyResponseDto>>,
            ApiResponse<regions_dtos::RegencyResponseDto>,
            ApiResponse<Vec<regions_dtos::DistrictResponseDto>>,
            ApiResponse<regions_dtos::DistrictResponseDto>,
            ApiResponse<Vec<regions_dtos::VillageResponseDto>>,
            ApiResponse<regions_dtos::VillageResponseDto>,
            // Files
            files_dtos::UploadFileDto,
            files_dtos::FileVisibilityDto,
            files_dtos::FileResponseDto,
            files_dtos::DeleteFileByUrlDto,
            files_dtos::DeleteFileResponseDto,
            ApiResponse<files_dtos::FileResponseDto>,
            ApiResponse<files_dtos::DeleteFileResponseDto>,
            // Expectations
            expectations_dtos::CreateExpectationDto,
            expectations_dtos::ExpectationResponseDto,
            ApiResponse<expectations_dtos::ExpectationResponseDto>,
            // Contributors
            contributors_dtos::CreateContributorDto,
            contributors_dtos::ContributorResponseDto,
            ApiResponse<contributors_dtos::ContributorResponseDto>,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User profile management"),
        (name = "regions", description = "Indonesian administrative regions (provinces, regencies, districts, villages)"),
        (name = "files", description = "File upload and management"),
        (name = "expectations", description = "User expectations from landing page (public)"),
        (name = "contributors", description = "Contributor registration (public)"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Balungpisah API",
        version = "0.1.0",
        description = "API documentation for Balungpisah",
    )
)]
pub struct ApiDoc;

/// Adds Bearer JWT security scheme to OpenAPI spec
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Modifier to override OpenAPI info from config
pub struct SwaggerInfoModifier {
    pub title: String,
    pub version: String,
    pub description: String,
}

impl Modify for SwaggerInfoModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi.info.title = self.title.clone();
        openapi.info.version = self.version.clone();
        openapi.info.description = Some(self.description.clone());
    }
}
