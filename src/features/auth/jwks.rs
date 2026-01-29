use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

struct JwksCache {
    keys: HashMap<String, DecodingKey>,
    last_fetched: Instant,
}

pub struct JwksClient {
    issuer_url: String,
    client: reqwest::Client,
    cache: Arc<RwLock<Option<JwksCache>>>,
    cache_ttl: Duration,
}

impl JwksClient {
    pub fn new(issuer_url: &str, cache_ttl: Duration) -> Self {
        Self {
            issuer_url: issuer_url.to_string(),
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(None)),
            cache_ttl,
        }
    }

    pub async fn get_key(&self, kid: &str) -> Result<DecodingKey, JwksError> {
        // Try to get from cache first
        {
            let cache = self.cache.read().await;
            if let Some(ref cached) = *cache {
                if cached.last_fetched.elapsed() < self.cache_ttl {
                    if let Some(key) = cached.keys.get(kid) {
                        return Ok(key.clone());
                    }
                }
            }
        }

        // Cache miss or expired - fetch new keys
        self.fetch_jwks().await?;

        // Try again from cache
        let cache = self.cache.read().await;
        if let Some(ref cached) = *cache {
            cached
                .keys
                .get(kid)
                .cloned()
                .ok_or(JwksError::KeyNotFound(kid.to_string()))
        } else {
            Err(JwksError::KeyNotFound(kid.to_string()))
        }
    }

    async fn fetch_jwks(&self) -> Result<(), JwksError> {
        let jwks_url = format!("{}/jwks", self.issuer_url);

        let response = self
            .client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| JwksError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(JwksError::FetchError(format!(
                "Failed to fetch JWKS: HTTP {}",
                response.status()
            )));
        }

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| JwksError::ParseError(e.to_string()))?;

        let mut keys = HashMap::new();

        for jwk in jwks.keys {
            if jwk.kty == "RSA" {
                let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
                    .map_err(|e| JwksError::KeyConversionError(e.to_string()))?;
                keys.insert(jwk.kid, decoding_key);
            }
        }

        let mut cache = self.cache.write().await;
        *cache = Some(JwksCache {
            keys,
            last_fetched: Instant::now(),
        });

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JwksError {
    #[error("Failed to fetch JWKS: {0}")]
    FetchError(String),

    #[error("Failed to parse JWKS: {0}")]
    ParseError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Failed to convert key: {0}")]
    KeyConversionError(String),
}
