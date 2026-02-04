use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json;
use std::str::FromStr;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{CreateReportLocation, GeocodingSource};

/// The level at which geocoding succeeded
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeocodingLevel {
    /// Query succeeded at "Village, District" level
    Village,
    /// Query succeeded at "District, Regency" level
    District,
    /// Query succeeded at "Regency, Province" level
    Regency,
}

/// Result of cascading geocoding with the level that succeeded
#[derive(Debug)]
pub struct CascadingGeocodingResult {
    pub response: NominatimResponse,
    pub level: GeocodingLevel,
}

/// Location names extracted by LLM
#[derive(Debug, Default)]
pub struct LocationNames<'a> {
    pub street: Option<&'a str>,
    pub village: Option<&'a str>,
    pub district: Option<&'a str>,
    pub regency: Option<&'a str>,
    pub province: Option<&'a str>,
}

/// Nominatim API response structure
#[derive(Debug, Deserialize)]
pub struct NominatimResponse {
    pub lat: String,
    pub lon: String,
    pub display_name: String,
    pub osm_id: Option<i64>,
    pub osm_type: Option<String>,
    pub importance: Option<f64>,
    pub address: Option<NominatimAddress>,
    pub boundingbox: Option<Vec<String>>,
}

/// Nominatim address components
///
/// Field mapping for Indonesia (INCONSISTENT across regions!):
///
/// **Jawa pattern:**
/// - `state` → Province (e.g., "Jawa Barat")
/// - `county` → Kabupaten/Regency (e.g., "Bandung")
/// - `municipality` → Kecamatan/District (e.g., "Cibiru")
/// - `village` → Desa/Kelurahan (e.g., "Cisurupan")
///
/// **Sumatera pattern:**
/// - `state` → Province (e.g., "Sumatera Utara")
/// - `region` → Kabupaten/Regency (e.g., "Deli Serdang") ← DIFFERENT!
/// - NO municipality field for Kecamatan!
/// - `village` → Desa (e.g., "Bidar Alam")
///
/// Other fields:
/// - `city` → City/Kota (only for cities, not kabupaten)
/// - `town` → Can be kecamatan or small town (context-dependent)
/// - `suburb` → Urban sub-district
#[derive(Debug, Deserialize)]
pub struct NominatimAddress {
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub town: Option<String>,
    pub village: Option<String>,
    pub county: Option<String>,       // Kabupaten/Regency (Jawa pattern)
    pub region: Option<String>,       // Kabupaten/Regency (Sumatera pattern) or Island name
    pub municipality: Option<String>, // Kecamatan/District (may be missing in some regions)
    pub state: Option<String>,
    pub postcode: Option<String>,
    pub country_code: Option<String>,
}

impl NominatimAddress {
    /// Get regency/kabupaten name
    ///
    /// Handles inconsistent OSM data across Indonesia:
    /// - Jawa: uses `county` for kabupaten (e.g., "Bandung")
    /// - Sumatera: uses `region` for kabupaten (e.g., "Deli Serdang")
    /// - Falls back to `city` for kota (same administrative level)
    pub fn get_regency(&self) -> Option<String> {
        // Jawa pattern: county is kabupaten
        if let Some(county) = &self.county {
            return Some(county.clone());
        }
        // Sumatera pattern: region is kabupaten (when county is missing)
        // Note: In Jawa, region = island name ("Jawa"), so only use if county is None
        if let Some(region) = &self.region {
            return Some(region.clone());
        }
        // Fallback: city for kota
        self.city.clone()
    }

    /// Get district/kecamatan name
    ///
    /// Note: Kecamatan data is often missing in Sumatera OSM data!
    /// - Jawa: uses `municipality` for kecamatan
    /// - Sumatera: often NO kecamatan field available
    /// - Falls back to `town` which can sometimes be a kecamatan
    pub fn get_district(&self) -> Option<String> {
        self.municipality.clone().or_else(|| self.town.clone())
    }

    /// Get village/desa/kelurahan name
    pub fn get_village(&self) -> Option<String> {
        self.village.clone()
    }

    /// Get city name for display purposes
    /// Returns the most specific city-level name available
    pub fn get_city_display(&self) -> Option<String> {
        self.city
            .clone()
            .or_else(|| self.county.clone())
            .or_else(|| self.region.clone())
            .or_else(|| self.town.clone())
    }
}

/// Service for geocoding addresses using Nominatim
pub struct GeocodingService {
    client: reqwest::Client,
    base_url: String,
}

impl GeocodingService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("BalungpisahCore/1.0 (citizen-report-system)")
                .build()
                .expect("Failed to build HTTP client"),
            base_url: "https://nominatim.openstreetmap.org".to_string(),
        }
    }

    /// Geocode a raw location input using Nominatim free-form query
    pub async fn geocode(&self, raw_input: &str) -> Result<Option<NominatimResponse>> {
        let url = format!(
            "{}/search?q={}&format=json&addressdetails=1&limit=1&countrycodes=id",
            self.base_url,
            urlencoding::encode(raw_input)
        );

        tracing::debug!("Geocoding (free-form): {} -> {}", raw_input, url);

        self.execute_request(&url).await
    }

    /// Geocode using cascading free-form queries for Indonesia
    ///
    /// Tries progressively less specific queries until a result is found.
    /// Uses maximum 2 location parts per query (Nominatim works best this way).
    ///
    /// Returns both the Nominatim response AND the level at which geocoding succeeded.
    /// This allows the caller to only store region IDs appropriate to the query level:
    /// - Village level: store village_id, district_id, regency_id, province_id
    /// - District level: store district_id, regency_id, province_id (no village_id)
    /// - Regency level: store regency_id, province_id (no district_id, village_id)
    ///
    /// Cascade order:
    /// 1. "Village, District" (e.g., "Cisurupan, Cibiru")
    /// 2. "District, Regency" (e.g., "Cibiru, Bandung")
    /// 3. "Regency, Province" (e.g., "Bandung, Jawa Barat")
    ///
    /// Note: "Indonesia" suffix not needed because countrycodes=id is set.
    /// Street names are NOT included (unreliable in OpenStreetMap).
    pub async fn geocode_cascading(
        &self,
        village: Option<&str>,
        district: Option<&str>,
        regency: Option<&str>,
        province: Option<&str>,
    ) -> Result<Option<CascadingGeocodingResult>> {
        // Build list of queries with their levels, from most specific to least specific
        // Maximum 2 parts per query for best Nominatim results
        let mut queries: Vec<(String, GeocodingLevel)> = Vec::new();

        // Level 1: Village, District (most specific)
        if let (Some(v), Some(d)) = (village, district) {
            queries.push((format!("{}, {}", v, d), GeocodingLevel::Village));
        }

        // Level 2: District, Regency
        if let (Some(d), Some(r)) = (district, regency) {
            queries.push((format!("{}, {}", d, r), GeocodingLevel::District));
        }

        // Level 3: Regency, Province (last resort with location)
        if let (Some(r), Some(p)) = (regency, province) {
            queries.push((format!("{}, {}", r, p), GeocodingLevel::Regency));
        }

        // Try each query in order until we get a result
        for (i, (query, level)) in queries.iter().enumerate() {
            tracing::info!(
                "Geocoding attempt {}/{} ({:?}): {}",
                i + 1,
                queries.len(),
                level,
                query
            );

            let result = self.geocode(query).await?;

            if let Some(response) = result {
                tracing::info!(
                    "Geocoding successful at {:?} level (attempt {}): {}",
                    level,
                    i + 1,
                    query
                );
                return Ok(Some(CascadingGeocodingResult {
                    response,
                    level: *level,
                }));
            }

            tracing::debug!("Geocoding attempt {} returned no results", i + 1);
        }

        tracing::warn!(
            "All geocoding attempts failed for: village={:?}, district={:?}, regency={:?}, province={:?}",
            village,
            district,
            regency,
            province
        );

        Ok(None)
    }

    /// Execute HTTP request to Nominatim and parse response
    async fn execute_request(&self, url: &str) -> Result<Option<NominatimResponse>> {
        // Log request details
        tracing::debug!(
            target: "nominatim",
            request_url = %url,
            "OpenStreetMap Nominatim API request"
        );

        let response = self.client.get(url).send().await.map_err(|e| {
            tracing::error!(
                target: "nominatim",
                error = %e,
                request_url = %url,
                "Nominatim HTTP request failed"
            );
            AppError::ExternalServiceError(format!("Nominatim request failed: {}", e))
        })?;

        let status = response.status();

        if !status.is_success() {
            tracing::warn!(
                target: "nominatim",
                status_code = %status,
                request_url = %url,
                "Nominatim returned error status"
            );
            return Ok(None);
        }

        // Get the response body as text for logging
        let body_text = response.text().await.map_err(|e| {
            tracing::error!(
                target: "nominatim",
                error = %e,
                "Failed to read Nominatim response body"
            );
            AppError::ExternalServiceError(format!("Failed to read Nominatim response: {}", e))
        })?;

        // Log raw response
        tracing::debug!(
            target: "nominatim",
            status_code = %status,
            response_body = %body_text,
            "OpenStreetMap Nominatim API response"
        );

        // Parse the JSON response
        let results: Vec<NominatimResponse> = serde_json::from_str(&body_text).map_err(|e| {
            tracing::error!(
                target: "nominatim",
                error = %e,
                response_body = %body_text,
                "Failed to parse Nominatim JSON response"
            );
            AppError::ExternalServiceError(format!("Failed to parse Nominatim response: {}", e))
        })?;

        if results.is_empty() {
            tracing::debug!(
                target: "nominatim",
                "Nominatim returned empty results"
            );
            return Ok(None);
        }

        let result = results.into_iter().next();

        // Log the parsed result with structured fields
        if let Some(ref r) = result {
            let addr = r.address.as_ref();
            tracing::debug!(
                target: "nominatim",
                display_name = %r.display_name,
                lat = %r.lat,
                lon = %r.lon,
                osm_id = ?r.osm_id,
                osm_type = ?r.osm_type,
                state = ?addr.and_then(|a| a.state.as_ref()),
                county = ?addr.and_then(|a| a.county.as_ref()),
                city = ?addr.and_then(|a| a.city.as_ref()),
                municipality = ?addr.and_then(|a| a.municipality.as_ref()),
                village = ?addr.and_then(|a| a.village.as_ref()),
                "Nominatim geocoding result"
            );
        }

        Ok(result)
    }

    /// Convert Nominatim response to CreateReportLocation
    ///
    /// # Arguments
    /// * `report_id` - Report UUID
    /// * `raw_input` - Raw location text from user
    /// * `response` - Nominatim geocoding response (for lat/lon)
    /// * `names` - Location names extracted by LLM
    ///
    /// Note: `display_name` is built from LLM-extracted fields, not Nominatim's display_name
    pub fn to_create_location(
        &self,
        report_id: uuid::Uuid,
        raw_input: String,
        response: Option<NominatimResponse>,
        names: LocationNames,
    ) -> CreateReportLocation {
        // Build display_name from LLM-extracted location fields
        // Format: "Jalan X, Desa Y, Kecamatan Z, Kabupaten A, Provinsi B"
        let display_name = Self::build_display_name(
            names.street,
            names.village,
            names.district,
            names.regency,
            names.province,
        );

        match response {
            Some(r) => {
                let lat = r.lat.parse().ok();
                let lon = r.lon.parse().ok();
                let bounding_box = r.boundingbox.as_ref().map(|bb| {
                    serde_json::json!(bb
                        .iter()
                        .filter_map(|s| s.parse::<f64>().ok())
                        .collect::<Vec<_>>())
                });

                let address = r.address.as_ref();
                let geocoding_score = r
                    .importance
                    .and_then(|i| Decimal::from_str(&format!("{:.2}", i)).ok());

                CreateReportLocation {
                    report_id,
                    raw_input,
                    display_name,
                    lat,
                    lon,
                    osm_id: r.osm_id,
                    osm_type: r.osm_type,
                    road: address.and_then(|a| a.road.clone()),
                    neighbourhood: address.and_then(|a| a.neighbourhood.clone()),
                    suburb: address.and_then(|a| a.suburb.clone()),
                    city: address.and_then(|a| a.get_city_display()),
                    state: address.and_then(|a| a.state.clone()),
                    postcode: address.and_then(|a| a.postcode.clone()),
                    country_code: address.and_then(|a| a.country_code.clone()),
                    bounding_box,
                    geocoding_source: GeocodingSource::Nominatim,
                    geocoding_score,
                    // Region FKs will be resolved by RegionLookupService
                    province_id: None,
                    regency_id: None,
                    district_id: None,
                    village_id: None,
                }
            }
            None => CreateReportLocation {
                report_id,
                raw_input,
                display_name,
                lat: None,
                lon: None,
                osm_id: None,
                osm_type: None,
                road: None,
                neighbourhood: None,
                suburb: None,
                city: None,
                state: None,
                postcode: None,
                country_code: None,
                bounding_box: None,
                geocoding_source: GeocodingSource::Nominatim,
                geocoding_score: None,
                // Region FKs will be resolved by RegionLookupService
                province_id: None,
                regency_id: None,
                district_id: None,
                village_id: None,
            },
        }
    }

    /// Build human-readable display name from LLM-extracted location fields
    ///
    /// Format: "Street, Village, District, Regency, Province"
    /// Only includes fields that are present (skips None values)
    /// No prefixes added - just the location names joined with commas
    ///
    /// # Examples
    /// ```
    /// // Full address
    /// build_display_name(
    ///     Some("Jalan Sudirman"),
    ///     Some("Cisurupan"),
    ///     Some("Cibiru"),
    ///     Some("Bandung"),
    ///     Some("Jawa Barat")
    /// ) // => "Jalan Sudirman, Cisurupan, Cibiru, Bandung, Jawa Barat"
    ///
    /// // Partial address (only regency and province)
    /// build_display_name(None, None, None, Some("Bandung"), Some("Jawa Barat"))
    /// // => "Bandung, Jawa Barat"
    ///
    /// // Only province
    /// build_display_name(None, None, None, None, Some("Jawa Barat"))
    /// // => "Jawa Barat"
    /// ```
    fn build_display_name(
        street: Option<&str>,
        village: Option<&str>,
        district: Option<&str>,
        regency: Option<&str>,
        province: Option<&str>,
    ) -> Option<String> {
        let parts: Vec<&str> = [street, village, district, regency, province]
            .into_iter()
            .flatten()
            .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }
}

impl Default for GeocodingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_address() -> NominatimAddress {
        NominatimAddress {
            road: None,
            neighbourhood: None,
            suburb: None,
            city: None,
            town: None,
            village: None,
            county: None,
            region: None,
            municipality: None,
            state: None,
            postcode: None,
            country_code: None,
        }
    }

    #[test]
    fn test_nominatim_address_get_city_display() {
        // Test with city (Kota)
        let mut addr = make_address();
        addr.city = Some("Jakarta".to_string());
        assert_eq!(addr.get_city_display(), Some("Jakarta".to_string()));

        // Test with county (Kabupaten) - fallback when no city
        let mut addr2 = make_address();
        addr2.county = Some("Bandung".to_string());
        assert_eq!(addr2.get_city_display(), Some("Bandung".to_string()));

        // Test city takes precedence over county
        let mut addr3 = make_address();
        addr3.city = Some("Jakarta".to_string());
        addr3.county = Some("Bandung".to_string());
        assert_eq!(addr3.get_city_display(), Some("Jakarta".to_string()));
    }

    #[test]
    fn test_nominatim_address_get_regency() {
        // Test with county (Jawa pattern - Kabupaten)
        let mut addr = make_address();
        addr.county = Some("Bandung".to_string());
        assert_eq!(addr.get_regency(), Some("Bandung".to_string()));

        // Test with region (Sumatera pattern - Kabupaten when county is missing)
        let mut addr_sumatera = make_address();
        addr_sumatera.region = Some("Deli Serdang".to_string());
        assert_eq!(
            addr_sumatera.get_regency(),
            Some("Deli Serdang".to_string())
        );

        // Test with city (Kota) - fallback when both county and region are missing
        let mut addr2 = make_address();
        addr2.city = Some("Jakarta".to_string());
        assert_eq!(addr2.get_regency(), Some("Jakarta".to_string()));

        // Test county takes precedence over region (Jawa pattern)
        // In Jawa, region = island name "Jawa", county = kabupaten
        let mut addr3 = make_address();
        addr3.county = Some("Bandung".to_string());
        addr3.region = Some("Jawa".to_string());
        assert_eq!(addr3.get_regency(), Some("Bandung".to_string()));

        // Test county takes precedence over city
        let mut addr4 = make_address();
        addr4.county = Some("Bandung".to_string());
        addr4.city = Some("Jakarta".to_string());
        assert_eq!(addr4.get_regency(), Some("Bandung".to_string()));
    }

    #[test]
    fn test_nominatim_address_get_district() {
        // Test with municipality (Kecamatan)
        let mut addr = make_address();
        addr.municipality = Some("Cibiru".to_string());
        assert_eq!(addr.get_district(), Some("Cibiru".to_string()));

        // Test with town - fallback
        let mut addr2 = make_address();
        addr2.town = Some("Menteng".to_string());
        assert_eq!(addr2.get_district(), Some("Menteng".to_string()));
    }

    #[test]
    fn test_nominatim_address_get_village() {
        let mut addr = make_address();
        addr.village = Some("Cisurupan".to_string());
        assert_eq!(addr.get_village(), Some("Cisurupan".to_string()));
    }

    #[test]
    fn test_build_display_name_full_address() {
        let display_name = GeocodingService::build_display_name(
            Some("Jalan Sudirman"),
            Some("Cisurupan"),
            Some("Cibiru"),
            Some("Bandung"),
            Some("Jawa Barat"),
        );

        assert_eq!(
            display_name,
            Some("Jalan Sudirman, Cisurupan, Cibiru, Bandung, Jawa Barat".to_string())
        );
    }

    #[test]
    fn test_build_display_name_partial_regency_province() {
        let display_name = GeocodingService::build_display_name(
            None,
            None,
            None,
            Some("Bandung"),
            Some("Jawa Barat"),
        );

        assert_eq!(display_name, Some("Bandung, Jawa Barat".to_string()));
    }

    #[test]
    fn test_build_display_name_district_regency_province() {
        let display_name = GeocodingService::build_display_name(
            None,
            None,
            Some("Cibiru"),
            Some("Bandung"),
            Some("Jawa Barat"),
        );

        assert_eq!(
            display_name,
            Some("Cibiru, Bandung, Jawa Barat".to_string())
        );
    }

    #[test]
    fn test_build_display_name_village_district_regency() {
        let display_name = GeocodingService::build_display_name(
            None,
            Some("Cisurupan"),
            Some("Cibiru"),
            Some("Bandung"),
            None,
        );

        assert_eq!(display_name, Some("Cisurupan, Cibiru, Bandung".to_string()));
    }

    #[test]
    fn test_build_display_name_empty() {
        let display_name = GeocodingService::build_display_name(None, None, None, None, None);

        assert_eq!(display_name, None);
    }

    #[test]
    fn test_build_display_name_only_province() {
        let display_name =
            GeocodingService::build_display_name(None, None, None, None, Some("Jawa Barat"));

        assert_eq!(display_name, Some("Jawa Barat".to_string()));
    }

    #[test]
    fn test_build_display_name_with_street_only() {
        let display_name =
            GeocodingService::build_display_name(Some("Jalan Merdeka"), None, None, None, None);

        assert_eq!(display_name, Some("Jalan Merdeka".to_string()));
    }

    #[test]
    fn test_build_display_name_no_street() {
        let display_name = GeocodingService::build_display_name(
            None,
            Some("Tegallega"),
            Some("Bogor Tengah"),
            Some("Bogor"),
            Some("Jawa Barat"),
        );

        assert_eq!(
            display_name,
            Some("Tegallega, Bogor Tengah, Bogor, Jawa Barat".to_string())
        );
    }
}
