use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::features::admin::{dtos as admin_dtos, handlers as admin_handlers};
use crate::features::auth;
use crate::features::categories::{dtos as categories_dtos, handlers as categories_handlers};
use crate::features::citizen_report_agent::{
    dtos as citizen_agent_dtos, handlers as citizen_agent_handlers,
};
use crate::features::contributors::{dtos as contributors_dtos, handlers as contributors_handlers};
use crate::features::dashboard::{dtos as dashboard_dtos, handlers as dashboard_handlers};
use crate::features::expectations::{dtos as expectations_dtos, handlers as expectations_handlers};
use crate::features::files::{dtos as files_dtos, handlers as files_handlers};
use crate::features::prompts::{dtos as prompts_dtos, handlers as prompts_handlers};
use crate::features::rate_limits::{dtos as rate_limits_dtos, handlers as rate_limits_handlers};
use crate::features::regions::{dtos as regions_dtos, handlers as regions_handlers};
use crate::features::reports::{
    dtos as reports_dtos, handlers as reports_handlers, models as reports_models,
};
use crate::features::tickets::{
    dtos as tickets_dtos, handlers as tickets_handlers, models as tickets_models,
};
use crate::features::users::{dtos as users_dtos, handlers::profile_handler};
use crate::shared::types::{ApiResponse, Meta};

#[derive(OpenApi)]
#[openapi(
    paths(
        // Auth
        auth::handlers::register,
        auth::handlers::login,
        auth::handlers::refresh_token,
        auth::handlers::get_me,
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
        // Categories (public)
        categories_handlers::list_categories,
        categories_handlers::get_category,
        // Tickets (protected)
        tickets_handlers::list_tickets,
        tickets_handlers::get_ticket,
        tickets_handlers::get_ticket_by_reference,
        // Citizen Report Agent
        citizen_agent_handlers::chat_handler::chat_stream,
        citizen_agent_handlers::chat_handler::chat_sync,
        citizen_agent_handlers::conversation_handler::list_threads,
        citizen_agent_handlers::conversation_handler::get_thread,
        citizen_agent_handlers::conversation_handler::list_messages,
        citizen_agent_handlers::attachment_handler::upload_attachment,
        citizen_agent_handlers::attachment_handler::list_attachments,
        citizen_agent_handlers::attachment_handler::count_attachments,
        citizen_agent_handlers::attachment_handler::delete_attachment,
        citizen_agent_handlers::rate_limit_handler::get_user_rate_limit,
        // Rate Limits (Admin)
        rate_limits_handlers::rate_limit_config_handler::list_rate_limit_configs,
        rate_limits_handlers::rate_limit_config_handler::get_rate_limit_config,
        rate_limits_handlers::rate_limit_config_handler::update_rate_limit_config,
        // Prompts (Super Admin)
        prompts_handlers::prompt_handler::create_prompt,
        prompts_handlers::prompt_handler::get_prompt,
        prompts_handlers::prompt_handler::list_prompts,
        prompts_handlers::prompt_handler::update_prompt,
        prompts_handlers::prompt_handler::delete_prompt,
        // Admin
        admin_handlers::list_expectations,
        admin_handlers::get_expectation,
        admin_handlers::list_reports,
        admin_handlers::get_report,
        admin_handlers::list_contributors,
        admin_handlers::get_contributor,
        admin_handlers::list_tickets,
        admin_handlers::get_ticket,
        // Reports
        reports_handlers::report_handler::list_reports,
        reports_handlers::report_handler::get_report,
        reports_handlers::report_handler::update_report_status,
        reports_handlers::report_handler::list_clusters,
        reports_handlers::report_handler::get_cluster,
        // Dashboard (public)
        dashboard_handlers::dashboard_handler::get_summary,
        dashboard_handlers::dashboard_handler::list_reports,
        dashboard_handlers::dashboard_handler::get_report,
        dashboard_handlers::dashboard_handler::get_by_location,
        dashboard_handlers::dashboard_handler::get_by_category,
        dashboard_handlers::dashboard_handler::get_by_tag,
        dashboard_handlers::dashboard_handler::get_recent,
        dashboard_handlers::dashboard_handler::get_map,
        dashboard_handlers::dashboard_handler::get_map_data,
    ),
    components(
        schemas(
            // Shared
            Meta,
            // Auth
            auth::dto::MeResponseDto,
            auth::model::AuthenticatedUser,
            auth::dtos::RegisterRequestDto,
            auth::dtos::LoginRequestDto,
            auth::dtos::RefreshTokenRequestDto,
            auth::dtos::RefreshTokenResponseDto,
            auth::dtos::AuthResponseDto,
            auth::dtos::AuthUserDto,
            ApiResponse<auth::dto::MeResponseDto>,
            ApiResponse<auth::dtos::AuthResponseDto>,
            ApiResponse<auth::dtos::RefreshTokenResponseDto>,
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
            // Categories
            categories_dtos::CategoryResponseDto,
            categories_dtos::CategoryTreeDto,
            ApiResponse<Vec<categories_dtos::CategoryResponseDto>>,
            ApiResponse<categories_dtos::CategoryResponseDto>,
            // Tickets
            tickets_models::TicketStatus,
            tickets_dtos::TicketResponseDto,
            ApiResponse<Vec<tickets_dtos::TicketResponseDto>>,
            ApiResponse<tickets_dtos::TicketResponseDto>,
            // Citizen Report Agent
            citizen_agent_dtos::ChatRequestDto,
            citizen_agent_dtos::ChatResponseDto,
            citizen_agent_dtos::ThreadResponseDto,
            citizen_agent_dtos::ThreadDetailDto,
            citizen_agent_dtos::MessageResponseDto,
            citizen_agent_dtos::ThreadAttachmentResponseDto,
            citizen_agent_dtos::AttachmentCountDto,
            citizen_agent_dtos::UploadAttachmentDto,
            citizen_agent_dtos::DeleteAttachmentResponseDto,
            ApiResponse<citizen_agent_dtos::ChatResponseDto>,
            ApiResponse<Vec<citizen_agent_dtos::ThreadResponseDto>>,
            ApiResponse<citizen_agent_dtos::ThreadDetailDto>,
            ApiResponse<Vec<citizen_agent_dtos::MessageResponseDto>>,
            ApiResponse<citizen_agent_dtos::ThreadAttachmentResponseDto>,
            ApiResponse<Vec<citizen_agent_dtos::ThreadAttachmentResponseDto>>,
            ApiResponse<citizen_agent_dtos::AttachmentCountDto>,
            ApiResponse<citizen_agent_dtos::DeleteAttachmentResponseDto>,
            // Rate Limits
            rate_limits_dtos::RateLimitConfigResponseDto,
            rate_limits_dtos::UpdateRateLimitConfigDto,
            rate_limits_dtos::UserRateLimitStatusDto,
            ApiResponse<Vec<rate_limits_dtos::RateLimitConfigResponseDto>>,
            ApiResponse<rate_limits_dtos::RateLimitConfigResponseDto>,
            ApiResponse<rate_limits_dtos::UserRateLimitStatusDto>,
            // Prompts
            prompts_dtos::CreatePromptDto,
            prompts_dtos::UpdatePromptDto,
            prompts_dtos::PromptResponseDto,
            prompts_dtos::PromptQueryParams,
            ApiResponse<prompts_dtos::PromptResponseDto>,
            ApiResponse<Vec<prompts_dtos::PromptResponseDto>>,
            // Reports
            reports_models::ReportStatus,
            reports_models::ReportSeverity,
            reports_models::ReportTagType,
            reports_models::ClusterStatus,
            reports_models::GeocodingSource,
            reports_dtos::ReportCategoryDto,
            reports_dtos::ReportTagDto,
            reports_dtos::ReportResponseDto,
            reports_dtos::ReportDetailResponseDto,
            reports_dtos::ReportLocationResponseDto,
            reports_dtos::ReportClusterResponseDto,
            reports_dtos::ClusterDetailResponseDto,
            reports_dtos::UpdateReportStatusDto,
            ApiResponse<Vec<reports_dtos::ReportResponseDto>>,
            ApiResponse<reports_dtos::ReportDetailResponseDto>,
            ApiResponse<reports_dtos::ReportResponseDto>,
            ApiResponse<Vec<reports_dtos::ReportClusterResponseDto>>,
            ApiResponse<reports_dtos::ClusterDetailResponseDto>,
            // Dashboard (public)
            dashboard_dtos::PaginationMeta,
            dashboard_dtos::ReportCategoryInfo,
            dashboard_dtos::ReportLocationInfo,
            dashboard_dtos::DashboardReportDto,
            dashboard_dtos::DashboardReportDetailDto,
            dashboard_dtos::ProvinceReportSummary,
            dashboard_dtos::RegencyReportSummary,
            dashboard_dtos::DashboardLocationOverviewDto,
            dashboard_dtos::CategoryReportSummary,
            dashboard_dtos::DashboardCategoryOverviewDto,
            dashboard_dtos::TagReportSummary,
            dashboard_dtos::DashboardTagOverviewDto,
            dashboard_dtos::DashboardRecentDto,
            dashboard_dtos::MapReportMarker,
            dashboard_dtos::DashboardMapDto,
            dashboard_dtos::DashboardSummaryDto,
            ApiResponse<dashboard_dtos::DashboardSummaryDto>,
            ApiResponse<Vec<dashboard_dtos::DashboardReportDto>>,
            ApiResponse<dashboard_dtos::DashboardReportDetailDto>,
            ApiResponse<dashboard_dtos::DashboardLocationOverviewDto>,
            ApiResponse<dashboard_dtos::DashboardCategoryOverviewDto>,
            ApiResponse<dashboard_dtos::DashboardTagOverviewDto>,
            ApiResponse<dashboard_dtos::DashboardRecentDto>,
            ApiResponse<dashboard_dtos::DashboardMapDto>,
            // Dashboard map data (geospatial)
            dashboard_dtos::MapPointDto,
            dashboard_dtos::DashboardMapDataDto,
            ApiResponse<dashboard_dtos::DashboardMapDataDto>,
            // Admin
            admin_dtos::SortDirection,
            admin_dtos::ReportSortBy,
            admin_dtos::TicketSortBy,
            admin_dtos::AdminExpectationDto,
            admin_dtos::AdminReportDto,
            admin_dtos::AdminReportDetailDto,
            admin_dtos::AdminReportCategoryDto,
            admin_dtos::AdminReportLocationDto,
            admin_dtos::AdminReportAttachmentDto,
            admin_dtos::AdminContributorDto,
            admin_dtos::AdminContributorDetailDto,
            admin_dtos::AdminTicketDto,
            admin_dtos::AdminTicketDetailDto,
            ApiResponse<Vec<admin_dtos::AdminExpectationDto>>,
            ApiResponse<admin_dtos::AdminExpectationDto>,
            ApiResponse<Vec<admin_dtos::AdminReportDto>>,
            ApiResponse<admin_dtos::AdminReportDetailDto>,
            ApiResponse<Vec<admin_dtos::AdminContributorDto>>,
            ApiResponse<admin_dtos::AdminContributorDetailDto>,
            ApiResponse<Vec<admin_dtos::AdminTicketDto>>,
            ApiResponse<admin_dtos::AdminTicketDetailDto>,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User profile management"),
        (name = "regions", description = "Indonesian administrative regions (provinces, regencies, districts, villages)"),
        (name = "files", description = "File upload and management"),
        (name = "expectations", description = "User expectations from landing page (public)"),
        (name = "contributors", description = "Contributor registration (public)"),
        (name = "citizen-report-agent", description = "AI-powered citizen report assistant"),
        (name = "categories", description = "Report categories (public)"),
        (name = "tickets", description = "Citizen report tickets"),
        (name = "reports", description = "Citizen reports and clusters"),
        (name = "Dashboard", description = "Public dashboard for viewing reports"),
        (name = "rate-limits", description = "Rate limit configuration (admin only)"),
        (name = "admin", description = "Admin endpoints (super admin only)"),
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
