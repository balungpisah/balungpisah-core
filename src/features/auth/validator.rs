use super::model::{AuthenticatedUser, CustomClaims};
use crate::core::error::AppError;
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use super::jwks::JwksClient;

pub struct JwtValidator {
    jwks_client: Arc<JwksClient>,
    issuer: String,
    audience: String,
    leeway: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct Claims {
    // Standard JWT claims (validated by jsonwebtoken library)
    #[serde(rename = "jti", default)]
    _jti: Option<String>,
    sub: String,
    #[serde(rename = "iss")]
    _iss: String,
    #[serde(rename = "aud")]
    _aud: AudienceClaim,
    #[serde(rename = "iat")]
    _iat: u64,
    #[serde(rename = "exp")]
    _exp: u64,

    // Logto-specific claims (some may be optional for token exchange tokens)
    #[serde(default)]
    kind: Option<String>,
    #[serde(alias = "client_id", default)]
    _client_id: Option<String>,
    #[serde(rename = "accountId", default)]
    account_id: Option<String>,
    #[serde(rename = "sessionUid", default)]
    session_uid: Option<String>,

    // Custom claims - configure your own namespace in your OIDC provider
    #[serde(rename = "https://balungpisah.id/claims", default)]
    custom_claims: Option<CustomClaims>,
}

/// Audience can be either a single string or an array of strings
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum AudienceClaim {
    Single(String),
    Multiple(Vec<String>),
}

impl JwtValidator {
    pub fn new(
        jwks_client: Arc<JwksClient>,
        issuer: String,
        audience: String,
        leeway: Duration,
    ) -> Self {
        Self {
            jwks_client,
            issuer,
            audience,
            leeway: leeway.as_secs(),
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<AuthenticatedUser, AppError> {
        // Decode header to get kid
        let header = decode_header(token).map_err(|e| AppError::Auth(e.to_string()))?;

        let kid = header
            .kid
            .ok_or_else(|| AppError::Auth("Missing kid in token header".to_string()))?;

        // Get decoding key from JWKS
        let decoding_key = self
            .jwks_client
            .get_key(&kid)
            .await
            .map_err(|e| AppError::Auth(e.to_string()))?;

        // Validate algorithm from header
        if header.alg != Algorithm::RS256 {
            return Err(AppError::Auth(format!(
                "Unsupported algorithm: {:?}. Only RS256 is allowed",
                header.alg
            )));
        }

        // Setup validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.leeway = self.leeway;
        validation.validate_nbf = true; // Validate not-before claim

        // Decode and validate token
        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| AppError::Auth(e.to_string()))?;

        let claims = token_data.claims;

        // Additional validation: check token kind (if present)
        // Token exchange tokens may not have the 'kind' claim
        if let Some(kind) = &claims.kind {
            if kind != "AccessToken" {
                return Err(AppError::Auth("Token is not an access token".to_string()));
            }
        }

        // Validate this is a global token (if custom claims are present)
        // Extract roles from custom claims
        let roles = if let Some(custom) = &claims.custom_claims {
            if custom.token_type != "global" {
                return Err(AppError::Auth(
                    "This service requires a global access token".to_string(),
                ));
            }
            custom.roles.clone()
        } else {
            Vec::new()
        };

        // Convert to AuthenticatedUser
        // For token exchange tokens, account_id may not be present, use sub instead
        let account_id = claims.account_id.unwrap_or_else(|| claims.sub.clone());

        Ok(AuthenticatedUser {
            account_id,
            sub: claims.sub,
            session_uid: claims.session_uid,
            roles,
        })
    }
}
