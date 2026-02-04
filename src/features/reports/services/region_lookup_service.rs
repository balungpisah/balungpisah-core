use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};

/// Resolved region IDs from fuzzy matching
#[derive(Debug, Clone, Default)]
pub struct ResolvedRegions {
    pub province_id: Option<Uuid>,
    pub regency_id: Option<Uuid>,
    pub district_id: Option<Uuid>,
    pub village_id: Option<Uuid>,
}

/// Service for resolving region names to UUIDs
pub struct RegionLookupService {
    pool: PgPool,
}

impl RegionLookupService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolve location names to region IDs
    ///
    /// Uses fuzzy matching with ILIKE to find the best match.
    /// Resolves the full hierarchy: province → regency → district → village
    ///
    /// # Arguments
    /// * `city` - City/Regency/Kabupaten name
    /// * `state` - Province name
    /// * `district` - District/Kecamatan name (optional)
    /// * `village` - Village/Desa/Kelurahan name (optional)
    pub async fn resolve(
        &self,
        city: Option<&str>,
        state: Option<&str>,
        district: Option<&str>,
        village: Option<&str>,
    ) -> Result<ResolvedRegions> {
        let mut result = ResolvedRegions::default();

        // First try to resolve province from state
        if let Some(state_name) = state {
            result.province_id = self.find_province(state_name).await?;
        }

        // Then try to resolve regency from city
        if let Some(city_name) = city {
            // Try to find regency matching the city name
            if let Some((regency_id, province_id)) =
                self.find_regency(city_name, result.province_id).await?
            {
                result.regency_id = Some(regency_id);
                // If we found a regency but didn't have a province, use the regency's province
                if result.province_id.is_none() {
                    result.province_id = Some(province_id);
                }
            }
        }

        // Try to resolve district from municipality/kecamatan name
        if let Some(district_name) = district {
            if let Some((district_id, regency_id)) =
                self.find_district(district_name, result.regency_id).await?
            {
                result.district_id = Some(district_id);
                // If we found a district but didn't have a regency, use the district's regency
                if result.regency_id.is_none() {
                    result.regency_id = Some(regency_id);
                    // Also try to get the province from the regency
                    if result.province_id.is_none() {
                        result.province_id = self.get_province_from_regency(regency_id).await?;
                    }
                }
            }
        }

        // Try to resolve village from desa/kelurahan name
        if let Some(village_name) = village {
            if let Some((village_id, district_id)) =
                self.find_village(village_name, result.district_id).await?
            {
                result.village_id = Some(village_id);
                // If we found a village but didn't have a district, use the village's district
                if result.district_id.is_none() {
                    result.district_id = Some(district_id);
                    // Also try to backfill regency and province
                    if result.regency_id.is_none() {
                        if let Some(regency_id) =
                            self.get_regency_from_district(district_id).await?
                        {
                            result.regency_id = Some(regency_id);
                            if result.province_id.is_none() {
                                result.province_id =
                                    self.get_province_from_regency(regency_id).await?;
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(
            "Resolved regions: province={:?}, regency={:?}, district={:?}, village={:?}",
            result.province_id,
            result.regency_id,
            result.district_id,
            result.village_id
        );

        Ok(result)
    }

    /// Get province ID from regency ID
    async fn get_province_from_regency(&self, regency_id: Uuid) -> Result<Option<Uuid>> {
        let result = sqlx::query_scalar!(
            r#"SELECT province_id FROM regencies WHERE id = $1"#,
            regency_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get province from regency: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(result)
    }

    /// Get regency ID from district ID
    async fn get_regency_from_district(&self, district_id: Uuid) -> Result<Option<Uuid>> {
        let result = sqlx::query_scalar!(
            r#"SELECT regency_id FROM districts WHERE id = $1"#,
            district_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get regency from district: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(result)
    }

    /// Find province by name using fuzzy matching
    async fn find_province(&self, name: &str) -> Result<Option<Uuid>> {
        // Clean up the name - remove common prefixes
        let clean_name = name
            .trim()
            .replace("Provinsi ", "")
            .replace("Prov. ", "")
            .replace("Prov ", "");

        let result = sqlx::query_scalar!(
            r#"
            SELECT id
            FROM provinces
            WHERE name ILIKE $1
               OR name ILIKE $2
               OR name ILIKE '%' || $1 || '%'
            ORDER BY
                CASE
                    WHEN name ILIKE $1 THEN 1
                    WHEN name ILIKE $2 THEN 2
                    ELSE 3
                END
            LIMIT 1
            "#,
            &clean_name,
            name.trim()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to lookup province: {:?}", e);
            AppError::Database(e)
        })?;

        if result.is_some() {
            tracing::debug!("Resolved province '{}' -> {:?}", name, result);
        }

        Ok(result)
    }

    /// Find regency by name, optionally scoped to a province
    async fn find_regency(
        &self,
        name: &str,
        province_id: Option<Uuid>,
    ) -> Result<Option<(Uuid, Uuid)>> {
        // Clean up the name - remove common prefixes
        let clean_name = name
            .trim()
            .replace("Kabupaten ", "")
            .replace("Kab. ", "")
            .replace("Kab ", "")
            .replace("Kota ", "")
            .replace("Ko. ", "");

        let result = if let Some(prov_id) = province_id {
            // If we have a province, scope the search
            sqlx::query!(
                r#"
                SELECT r.id, r.province_id
                FROM regencies r
                WHERE r.province_id = $3
                  AND (r.name ILIKE $1
                       OR r.name ILIKE $2
                       OR r.name ILIKE '%' || $1 || '%')
                ORDER BY
                    CASE
                        WHEN r.name ILIKE $1 THEN 1
                        WHEN r.name ILIKE $2 THEN 2
                        ELSE 3
                    END
                LIMIT 1
                "#,
                &clean_name,
                name.trim(),
                prov_id
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lookup regency with province: {:?}", e);
                AppError::Database(e)
            })?
            .map(|r| (r.id, r.province_id))
        } else {
            // No province, search globally
            sqlx::query!(
                r#"
                SELECT r.id, r.province_id
                FROM regencies r
                WHERE r.name ILIKE $1
                   OR r.name ILIKE $2
                   OR r.name ILIKE '%' || $1 || '%'
                ORDER BY
                    CASE
                        WHEN r.name ILIKE $1 THEN 1
                        WHEN r.name ILIKE $2 THEN 2
                        ELSE 3
                    END
                LIMIT 1
                "#,
                &clean_name,
                name.trim()
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lookup regency: {:?}", e);
                AppError::Database(e)
            })?
            .map(|r| (r.id, r.province_id))
        };

        if result.is_some() {
            tracing::debug!(
                "Resolved regency '{}' (province: {:?}) -> {:?}",
                name,
                province_id,
                result
            );
        }

        Ok(result)
    }

    /// Find district by name within a regency
    async fn find_district(
        &self,
        name: &str,
        regency_id: Option<Uuid>,
    ) -> Result<Option<(Uuid, Uuid)>> {
        // Clean up the name
        let clean_name = name
            .trim()
            .replace("Kecamatan ", "")
            .replace("Kec. ", "")
            .replace("Kec ", "");

        let result = if let Some(reg_id) = regency_id {
            sqlx::query!(
                r#"
                SELECT d.id, d.regency_id
                FROM districts d
                WHERE d.regency_id = $3
                  AND (d.name ILIKE $1
                       OR d.name ILIKE $2
                       OR d.name ILIKE '%' || $1 || '%')
                ORDER BY
                    CASE
                        WHEN d.name ILIKE $1 THEN 1
                        WHEN d.name ILIKE $2 THEN 2
                        ELSE 3
                    END
                LIMIT 1
                "#,
                &clean_name,
                name.trim(),
                reg_id
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lookup district with regency: {:?}", e);
                AppError::Database(e)
            })?
            .map(|r| (r.id, r.regency_id))
        } else {
            sqlx::query!(
                r#"
                SELECT d.id, d.regency_id
                FROM districts d
                WHERE d.name ILIKE $1
                   OR d.name ILIKE $2
                   OR d.name ILIKE '%' || $1 || '%'
                ORDER BY
                    CASE
                        WHEN d.name ILIKE $1 THEN 1
                        WHEN d.name ILIKE $2 THEN 2
                        ELSE 3
                    END
                LIMIT 1
                "#,
                &clean_name,
                name.trim()
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lookup district: {:?}", e);
                AppError::Database(e)
            })?
            .map(|r| (r.id, r.regency_id))
        };

        Ok(result)
    }

    /// Find village by name within a district
    async fn find_village(
        &self,
        name: &str,
        district_id: Option<Uuid>,
    ) -> Result<Option<(Uuid, Uuid)>> {
        // Clean up the name
        let clean_name = name
            .trim()
            .replace("Kelurahan ", "")
            .replace("Kel. ", "")
            .replace("Desa ", "")
            .replace("Ds. ", "");

        let result = if let Some(dist_id) = district_id {
            sqlx::query!(
                r#"
                SELECT v.id, v.district_id
                FROM villages v
                WHERE v.district_id = $3
                  AND (v.name ILIKE $1
                       OR v.name ILIKE $2
                       OR v.name ILIKE '%' || $1 || '%')
                ORDER BY
                    CASE
                        WHEN v.name ILIKE $1 THEN 1
                        WHEN v.name ILIKE $2 THEN 2
                        ELSE 3
                    END
                LIMIT 1
                "#,
                &clean_name,
                name.trim(),
                dist_id
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lookup village with district: {:?}", e);
                AppError::Database(e)
            })?
            .map(|r| (r.id, r.district_id))
        } else {
            sqlx::query!(
                r#"
                SELECT v.id, v.district_id
                FROM villages v
                WHERE v.name ILIKE $1
                   OR v.name ILIKE $2
                   OR v.name ILIKE '%' || $1 || '%'
                ORDER BY
                    CASE
                        WHEN v.name ILIKE $1 THEN 1
                        WHEN v.name ILIKE $2 THEN 2
                        ELSE 3
                    END
                LIMIT 1
                "#,
                &clean_name,
                name.trim()
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lookup village: {:?}", e);
                AppError::Database(e)
            })?
            .map(|r| (r.id, r.district_id))
        };

        Ok(result)
    }
}
