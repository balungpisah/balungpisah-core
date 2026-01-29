use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::features::regions::models::{District, Province, Regency, Village};

/// Query parameters for searching regions
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct RegionSearchQuery {
    /// Search by name (case-insensitive, partial match)
    #[param(example = "jakarta")]
    pub search: Option<String>,
}

/// Response DTO for province data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProvinceResponseDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
}

impl From<Province> for ProvinceResponseDto {
    fn from(province: Province) -> Self {
        Self {
            id: province.id,
            code: province.code,
            name: province.name,
            lat: province.lat,
            lng: province.lng,
        }
    }
}

/// Response DTO for regency data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegencyResponseDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
    pub province_id: Uuid,
}

impl From<Regency> for RegencyResponseDto {
    fn from(regency: Regency) -> Self {
        Self {
            id: regency.id,
            code: regency.code,
            name: regency.name,
            lat: regency.lat,
            lng: regency.lng,
            province_id: regency.province_id,
        }
    }
}

/// Response DTO for district data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DistrictResponseDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
    pub regency_id: Uuid,
}

impl From<District> for DistrictResponseDto {
    fn from(district: District) -> Self {
        Self {
            id: district.id,
            code: district.code,
            name: district.name,
            lat: district.lat,
            lng: district.lng,
            regency_id: district.regency_id,
        }
    }
}

/// Response DTO for village data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VillageResponseDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
    pub district_id: Uuid,
}

impl From<Village> for VillageResponseDto {
    fn from(village: Village) -> Self {
        Self {
            id: village.id,
            code: village.code,
            name: village.name,
            lat: village.lat,
            lng: village.lng,
            district_id: village.district_id,
        }
    }
}
