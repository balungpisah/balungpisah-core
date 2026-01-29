use sqlx::PgPool;

use crate::core::error::{AppError, Result};
use crate::features::regions::models::{District, Province, Regency, Village};

/// Service for managing Indonesian administrative regions
pub struct RegionService {
    pool: PgPool,
}

impl RegionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== Province Methods ====================

    /// List all provinces with optional search
    pub async fn list_provinces(&self, search: Option<&str>) -> Result<Vec<Province>> {
        let provinces = match search {
            Some(term) if !term.is_empty() => {
                let search_pattern = format!("%{}%", term.to_lowercase());
                sqlx::query_as!(
                    Province,
                    r#"
                    SELECT id, code, name, lat, lng, created_at, updated_at
                    FROM provinces
                    WHERE LOWER(name) LIKE $1 OR code LIKE $1
                    ORDER BY code ASC
                    "#,
                    search_pattern
                )
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as!(
                    Province,
                    r#"
                    SELECT id, code, name, lat, lng, created_at, updated_at
                    FROM provinces
                    ORDER BY code ASC
                    "#
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| {
            tracing::error!("Failed to fetch provinces: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(provinces)
    }

    /// Get a province by its code
    pub async fn get_province_by_code(&self, code: &str) -> Result<Province> {
        let province = sqlx::query_as!(
            Province,
            r#"
            SELECT id, code, name, lat, lng, created_at, updated_at
            FROM provinces
            WHERE code = $1
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch province by code {}: {:?}", code, e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Province with code '{}' not found", code)))?;

        Ok(province)
    }

    // ==================== Regency Methods ====================

    /// List all regencies in a province with optional search
    pub async fn list_regencies_by_province_code(
        &self,
        province_code: &str,
        search: Option<&str>,
    ) -> Result<Vec<Regency>> {
        // First verify the province exists
        let province = self.get_province_by_code(province_code).await?;

        let regencies = match search {
            Some(term) if !term.is_empty() => {
                let search_pattern = format!("%{}%", term.to_lowercase());
                sqlx::query_as!(
                    Regency,
                    r#"
                    SELECT id, code, name, lat, lng, province_id, created_at, updated_at
                    FROM regencies
                    WHERE province_id = $1 AND (LOWER(name) LIKE $2 OR code LIKE $2)
                    ORDER BY code ASC
                    "#,
                    province.id,
                    search_pattern
                )
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as!(
                    Regency,
                    r#"
                    SELECT id, code, name, lat, lng, province_id, created_at, updated_at
                    FROM regencies
                    WHERE province_id = $1
                    ORDER BY code ASC
                    "#,
                    province.id
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| {
            tracing::error!(
                "Failed to fetch regencies for province {}: {:?}",
                province_code,
                e
            );
            AppError::Database(e)
        })?;

        Ok(regencies)
    }

    /// Get a regency by its code
    pub async fn get_regency_by_code(&self, code: &str) -> Result<Regency> {
        let regency = sqlx::query_as!(
            Regency,
            r#"
            SELECT id, code, name, lat, lng, province_id, created_at, updated_at
            FROM regencies
            WHERE code = $1
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch regency by code {}: {:?}", code, e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Regency with code '{}' not found", code)))?;

        Ok(regency)
    }

    /// Search regencies across all provinces
    pub async fn search_regencies(&self, search: &str) -> Result<Vec<Regency>> {
        let search_pattern = format!("%{}%", search.to_lowercase());
        let regencies = sqlx::query_as!(
            Regency,
            r#"
            SELECT id, code, name, lat, lng, province_id, created_at, updated_at
            FROM regencies
            WHERE LOWER(name) LIKE $1 OR code LIKE $1
            ORDER BY code ASC
            LIMIT 100
            "#,
            search_pattern
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to search regencies: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(regencies)
    }

    // ==================== District Methods ====================

    /// List all districts in a regency with optional search
    pub async fn list_districts_by_regency_code(
        &self,
        regency_code: &str,
        search: Option<&str>,
    ) -> Result<Vec<District>> {
        // First verify the regency exists
        let regency = self.get_regency_by_code(regency_code).await?;

        let districts = match search {
            Some(term) if !term.is_empty() => {
                let search_pattern = format!("%{}%", term.to_lowercase());
                sqlx::query_as!(
                    District,
                    r#"
                    SELECT id, code, name, lat, lng, regency_id, created_at, updated_at
                    FROM districts
                    WHERE regency_id = $1 AND (LOWER(name) LIKE $2 OR code LIKE $2)
                    ORDER BY code ASC
                    "#,
                    regency.id,
                    search_pattern
                )
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as!(
                    District,
                    r#"
                    SELECT id, code, name, lat, lng, regency_id, created_at, updated_at
                    FROM districts
                    WHERE regency_id = $1
                    ORDER BY code ASC
                    "#,
                    regency.id
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| {
            tracing::error!(
                "Failed to fetch districts for regency {}: {:?}",
                regency_code,
                e
            );
            AppError::Database(e)
        })?;

        Ok(districts)
    }

    /// Get a district by its code
    pub async fn get_district_by_code(&self, code: &str) -> Result<District> {
        let district = sqlx::query_as!(
            District,
            r#"
            SELECT id, code, name, lat, lng, regency_id, created_at, updated_at
            FROM districts
            WHERE code = $1
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch district by code {}: {:?}", code, e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("District with code '{}' not found", code)))?;

        Ok(district)
    }

    /// Search districts across all regencies
    pub async fn search_districts(&self, search: &str) -> Result<Vec<District>> {
        let search_pattern = format!("%{}%", search.to_lowercase());
        let districts = sqlx::query_as!(
            District,
            r#"
            SELECT id, code, name, lat, lng, regency_id, created_at, updated_at
            FROM districts
            WHERE LOWER(name) LIKE $1 OR code LIKE $1
            ORDER BY code ASC
            LIMIT 100
            "#,
            search_pattern
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to search districts: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(districts)
    }

    // ==================== Village Methods ====================

    /// List all villages in a district with optional search
    pub async fn list_villages_by_district_code(
        &self,
        district_code: &str,
        search: Option<&str>,
    ) -> Result<Vec<Village>> {
        // First verify the district exists
        let district = self.get_district_by_code(district_code).await?;

        let villages = match search {
            Some(term) if !term.is_empty() => {
                let search_pattern = format!("%{}%", term.to_lowercase());
                sqlx::query_as!(
                    Village,
                    r#"
                    SELECT id, code, name, lat, lng, district_id, created_at, updated_at
                    FROM villages
                    WHERE district_id = $1 AND (LOWER(name) LIKE $2 OR code LIKE $2)
                    ORDER BY code ASC
                    "#,
                    district.id,
                    search_pattern
                )
                .fetch_all(&self.pool)
                .await
            }
            _ => {
                sqlx::query_as!(
                    Village,
                    r#"
                    SELECT id, code, name, lat, lng, district_id, created_at, updated_at
                    FROM villages
                    WHERE district_id = $1
                    ORDER BY code ASC
                    "#,
                    district.id
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| {
            tracing::error!(
                "Failed to fetch villages for district {}: {:?}",
                district_code,
                e
            );
            AppError::Database(e)
        })?;

        Ok(villages)
    }

    /// Get a village by its code
    pub async fn get_village_by_code(&self, code: &str) -> Result<Village> {
        let village = sqlx::query_as!(
            Village,
            r#"
            SELECT id, code, name, lat, lng, district_id, created_at, updated_at
            FROM villages
            WHERE code = $1
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch village by code {}: {:?}", code, e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Village with code '{}' not found", code)))?;

        Ok(village)
    }

    /// Search villages across all districts
    pub async fn search_villages(&self, search: &str) -> Result<Vec<Village>> {
        let search_pattern = format!("%{}%", search.to_lowercase());
        let villages = sqlx::query_as!(
            Village,
            r#"
            SELECT id, code, name, lat, lng, district_id, created_at, updated_at
            FROM villages
            WHERE LOWER(name) LIKE $1 OR code LIKE $1
            ORDER BY code ASC
            LIMIT 100
            "#,
            search_pattern
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to search villages: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(villages)
    }
}
