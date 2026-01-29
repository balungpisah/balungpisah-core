use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::core::error::Result;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::regions::dtos::{
    DistrictResponseDto, ProvinceResponseDto, RegencyResponseDto, RegionSearchQuery,
    VillageResponseDto,
};
use crate::features::regions::services::RegionService;
use crate::shared::types::ApiResponse;

// ==================== Province Handlers ====================

/// List all provinces
#[utoipa::path(
    get,
    path = "/api/regions/provinces",
    params(RegionSearchQuery),
    responses(
        (status = 200, description = "List of provinces", body = ApiResponse<Vec<ProvinceResponseDto>>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn list_provinces(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<ProvinceResponseDto>>>> {
    let provinces = service.list_provinces(query.search.as_deref()).await?;
    let dtos: Vec<ProvinceResponseDto> = provinces.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

/// Get a province by code
#[utoipa::path(
    get,
    path = "/api/regions/provinces/{code}",
    params(
        ("code" = String, Path, description = "Province code (2 digits)")
    ),
    responses(
        (status = 200, description = "Province details", body = ApiResponse<ProvinceResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Province not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn get_province(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<ProvinceResponseDto>>> {
    let province = service.get_province_by_code(&code).await?;
    Ok(Json(ApiResponse::success(
        Some(province.into()),
        None,
        None,
    )))
}

/// List regencies in a province
#[utoipa::path(
    get,
    path = "/api/regions/provinces/{code}/regencies",
    params(
        ("code" = String, Path, description = "Province code (2 digits)"),
        RegionSearchQuery
    ),
    responses(
        (status = 200, description = "List of regencies in the province", body = ApiResponse<Vec<RegencyResponseDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Province not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn list_regencies_by_province(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<RegencyResponseDto>>>> {
    let regencies = service
        .list_regencies_by_province_code(&code, query.search.as_deref())
        .await?;
    let dtos: Vec<RegencyResponseDto> = regencies.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

// ==================== Regency Handlers ====================

/// Get a regency by code
#[utoipa::path(
    get,
    path = "/api/regions/regencies/{code}",
    params(
        ("code" = String, Path, description = "Regency code (format: XX.XX)")
    ),
    responses(
        (status = 200, description = "Regency details", body = ApiResponse<RegencyResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Regency not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn get_regency(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<RegencyResponseDto>>> {
    let regency = service.get_regency_by_code(&code).await?;
    Ok(Json(ApiResponse::success(Some(regency.into()), None, None)))
}

/// Search regencies across all provinces
#[utoipa::path(
    get,
    path = "/api/regions/regencies",
    params(RegionSearchQuery),
    responses(
        (status = 200, description = "List of regencies matching search", body = ApiResponse<Vec<RegencyResponseDto>>),
        (status = 400, description = "Search parameter required"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn search_regencies(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<RegencyResponseDto>>>> {
    let search_term = query
        .search
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            crate::core::error::AppError::BadRequest(
                "Search parameter 'search' is required".to_string(),
            )
        })?;

    let regencies = service.search_regencies(search_term).await?;
    let dtos: Vec<RegencyResponseDto> = regencies.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

/// List districts in a regency
#[utoipa::path(
    get,
    path = "/api/regions/regencies/{code}/districts",
    params(
        ("code" = String, Path, description = "Regency code (format: XX.XX)"),
        RegionSearchQuery
    ),
    responses(
        (status = 200, description = "List of districts in the regency", body = ApiResponse<Vec<DistrictResponseDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Regency not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn list_districts_by_regency(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<DistrictResponseDto>>>> {
    let districts = service
        .list_districts_by_regency_code(&code, query.search.as_deref())
        .await?;
    let dtos: Vec<DistrictResponseDto> = districts.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

// ==================== District Handlers ====================

/// Get a district by code
#[utoipa::path(
    get,
    path = "/api/regions/districts/{code}",
    params(
        ("code" = String, Path, description = "District code (format: XX.XX.XX)")
    ),
    responses(
        (status = 200, description = "District details", body = ApiResponse<DistrictResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "District not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn get_district(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<DistrictResponseDto>>> {
    let district = service.get_district_by_code(&code).await?;
    Ok(Json(ApiResponse::success(
        Some(district.into()),
        None,
        None,
    )))
}

/// Search districts across all regencies
#[utoipa::path(
    get,
    path = "/api/regions/districts",
    params(RegionSearchQuery),
    responses(
        (status = 200, description = "List of districts matching search", body = ApiResponse<Vec<DistrictResponseDto>>),
        (status = 400, description = "Search parameter required"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn search_districts(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<DistrictResponseDto>>>> {
    let search_term = query
        .search
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            crate::core::error::AppError::BadRequest(
                "Search parameter 'search' is required".to_string(),
            )
        })?;

    let districts = service.search_districts(search_term).await?;
    let dtos: Vec<DistrictResponseDto> = districts.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

/// List villages in a district
#[utoipa::path(
    get,
    path = "/api/regions/districts/{code}/villages",
    params(
        ("code" = String, Path, description = "District code (format: XX.XX.XX)"),
        RegionSearchQuery
    ),
    responses(
        (status = 200, description = "List of villages in the district", body = ApiResponse<Vec<VillageResponseDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "District not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn list_villages_by_district(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<VillageResponseDto>>>> {
    let villages = service
        .list_villages_by_district_code(&code, query.search.as_deref())
        .await?;
    let dtos: Vec<VillageResponseDto> = villages.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

// ==================== Village Handlers ====================

/// Get a village by code
#[utoipa::path(
    get,
    path = "/api/regions/villages/{code}",
    params(
        ("code" = String, Path, description = "Village code (format: XX.XX.XX.XXXX)")
    ),
    responses(
        (status = 200, description = "Village details", body = ApiResponse<VillageResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Village not found")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn get_village(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<VillageResponseDto>>> {
    let village = service.get_village_by_code(&code).await?;
    Ok(Json(ApiResponse::success(Some(village.into()), None, None)))
}

/// Search villages across all districts
#[utoipa::path(
    get,
    path = "/api/regions/villages",
    params(RegionSearchQuery),
    responses(
        (status = 200, description = "List of villages matching search", body = ApiResponse<Vec<VillageResponseDto>>),
        (status = 400, description = "Search parameter required"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "regions",
    security(("bearer_auth" = []))
)]
pub async fn search_villages(
    _user: AuthenticatedUser,
    State(service): State<Arc<RegionService>>,
    Query(query): Query<RegionSearchQuery>,
) -> Result<Json<ApiResponse<Vec<VillageResponseDto>>>> {
    let search_term = query
        .search
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            crate::core::error::AppError::BadRequest(
                "Search parameter 'search' is required".to_string(),
            )
        })?;

    let villages = service.search_villages(search_term).await?;
    let dtos: Vec<VillageResponseDto> = villages.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}
