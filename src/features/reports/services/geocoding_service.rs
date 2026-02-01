use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{CreateReportLocation, GeocodingSource};

/// Nominatim API response structure
#[derive(Debug, Deserialize)]
pub struct NominatimResponse {
    #[allow(dead_code)]
    pub place_id: i64,
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
#[derive(Debug, Deserialize)]
pub struct NominatimAddress {
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub town: Option<String>,
    pub village: Option<String>,
    pub state: Option<String>,
    pub postcode: Option<String>,
    pub country_code: Option<String>,
}

impl NominatimAddress {
    /// Get city, falling back to town or village
    pub fn get_city(&self) -> Option<String> {
        self.city
            .clone()
            .or_else(|| self.town.clone())
            .or_else(|| self.village.clone())
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

    /// Geocode using structured parameters for better accuracy
    ///
    /// Uses Nominatim's structured search with street, city, and state parameters.
    /// Falls back to free-form query if structured search returns no results.
    pub async fn geocode_structured(
        &self,
        query: Option<&str>,
        street: Option<&str>,
        city: Option<&str>,
        state: Option<&str>,
    ) -> Result<Option<NominatimResponse>> {
        // Try structured search first if we have street, city, or state
        if street.is_some() || city.is_some() || state.is_some() {
            let mut params = vec![
                ("format", "json".to_string()),
                ("addressdetails", "1".to_string()),
                ("limit", "1".to_string()),
                ("country", "Indonesia".to_string()),
            ];

            if let Some(s) = street {
                params.push(("street", s.to_string()));
            }
            if let Some(c) = city {
                params.push(("city", c.to_string()));
            }
            if let Some(st) = state {
                params.push(("state", st.to_string()));
            }

            let query_string = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&");

            let url = format!("{}/search?{}", self.base_url, query_string);
            tracing::debug!(
                "Geocoding (structured): {:?}/{:?}/{:?} -> {}",
                street,
                city,
                state,
                url
            );

            if let Some(result) = self.execute_request(&url).await? {
                return Ok(Some(result));
            }

            tracing::debug!("Structured search returned no results, trying free-form query");
        }

        // Fall back to free-form query if available
        if let Some(q) = query {
            let url = format!(
                "{}/search?q={}&format=json&addressdetails=1&limit=1&countrycodes=id",
                self.base_url,
                urlencoding::encode(q)
            );
            tracing::debug!("Geocoding (free-form fallback): {} -> {}", q, url);
            return self.execute_request(&url).await;
        }

        Ok(None)
    }

    /// Execute HTTP request to Nominatim and parse response
    async fn execute_request(&self, url: &str) -> Result<Option<NominatimResponse>> {
        let response = self.client.get(url).send().await.map_err(|e| {
            tracing::error!("Nominatim request failed: {:?}", e);
            AppError::ExternalServiceError(format!("Nominatim request failed: {}", e))
        })?;

        if !response.status().is_success() {
            tracing::warn!("Nominatim returned status: {}", response.status());
            return Ok(None);
        }

        let results: Vec<NominatimResponse> = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse Nominatim response: {:?}", e);
            AppError::ExternalServiceError(format!("Failed to parse Nominatim response: {}", e))
        })?;

        Ok(results.into_iter().next())
    }

    /// Convert Nominatim response to CreateReportLocation
    pub fn to_create_location(
        &self,
        report_id: uuid::Uuid,
        raw_input: String,
        response: Option<NominatimResponse>,
    ) -> CreateReportLocation {
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
                    display_name: Some(r.display_name),
                    lat,
                    lon,
                    osm_id: r.osm_id,
                    osm_type: r.osm_type,
                    road: address.and_then(|a| a.road.clone()),
                    neighbourhood: address.and_then(|a| a.neighbourhood.clone()),
                    suburb: address.and_then(|a| a.suburb.clone()),
                    city: address.and_then(|a| a.get_city()),
                    state: address.and_then(|a| a.state.clone()),
                    postcode: address.and_then(|a| a.postcode.clone()),
                    country_code: address.and_then(|a| a.country_code.clone()),
                    bounding_box,
                    geocoding_source: GeocodingSource::Nominatim,
                    geocoding_score,
                }
            }
            None => CreateReportLocation {
                report_id,
                raw_input,
                display_name: None,
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
            },
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

    #[test]
    fn test_nominatim_address_get_city() {
        let addr = NominatimAddress {
            road: None,
            neighbourhood: None,
            suburb: None,
            city: Some("Jakarta".to_string()),
            town: None,
            village: None,
            state: None,
            postcode: None,
            country_code: None,
        };
        assert_eq!(addr.get_city(), Some("Jakarta".to_string()));

        let addr2 = NominatimAddress {
            road: None,
            neighbourhood: None,
            suburb: None,
            city: None,
            town: Some("Bandung".to_string()),
            village: None,
            state: None,
            postcode: None,
            country_code: None,
        };
        assert_eq!(addr2.get_city(), Some("Bandung".to_string()));
    }
}
