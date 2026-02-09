//! One-time data migration script for PII encryption
//!
//! This script encrypts existing plaintext PII data in the database using AES-256-GCM
//! and generates blind indexes for searchable fields.
//!
//! Usage:
//!   cargo run --bin migrate_pii_encryption
//!
//! IMPORTANT: Take a database backup before running this script!

use anyhow::Result;
use base64::Engine;
use sqlx::postgres::PgPoolOptions;
use std::env;

// Re-use encryption service implementation inline since binaries can't import from lib
mod encryption {
    use aes_gcm::{
        aead::{Aead, AeadCore, KeyInit},
        Aes256Gcm, Nonce,
    };
    use base64::Engine;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    pub struct EncryptionService {
        cipher: Aes256Gcm,
        hmac_key: Vec<u8>,
    }

    impl EncryptionService {
        pub fn new(key: &[u8], hmac_key: &[u8]) -> Result<Self, String> {
            if key.len() != 32 {
                return Err("Encryption key must be 32 bytes (256 bits)".to_string());
            }

            if hmac_key.len() != 32 {
                return Err("HMAC key must be 32 bytes (256 bits)".to_string());
            }

            let cipher = Aes256Gcm::new_from_slice(key)
                .map_err(|e| format!("Failed to initialize cipher: {}", e))?;

            Ok(Self {
                cipher,
                hmac_key: hmac_key.to_vec(),
            })
        }

        pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
            let nonce = Aes256Gcm::generate_nonce(&mut rand::rngs::OsRng);

            let ciphertext = self
                .cipher
                .encrypt(&nonce, plaintext.as_bytes())
                .map_err(|e| format!("Encryption failed: {}", e))?;

            let nonce_b64 = base64::engine::general_purpose::STANDARD.encode(nonce);
            let ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(ciphertext);

            Ok(format!("{}:{}", nonce_b64, ciphertext_b64))
        }

        pub fn decrypt(&self, encrypted: &str) -> Result<String, String> {
            let parts: Vec<&str> = encrypted.split(':').collect();
            if parts.len() != 2 {
                return Err("Invalid encrypted format, expected 'nonce:ciphertext'".to_string());
            }

            let nonce_bytes = base64::engine::general_purpose::STANDARD
                .decode(parts[0])
                .map_err(|e| format!("Failed to decode nonce: {}", e))?;

            let ciphertext = base64::engine::general_purpose::STANDARD
                .decode(parts[1])
                .map_err(|e| format!("Failed to decode ciphertext: {}", e))?;

            #[allow(deprecated)]
            let nonce = Nonce::from_slice(&nonce_bytes);

            let plaintext = self
                .cipher
                .decrypt(nonce, ciphertext.as_ref())
                .map_err(|e| format!("Decryption failed: {}", e))?;

            String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8: {}", e))
        }

        pub fn blind_index(&self, value: &str) -> String {
            let normalized = value.trim().to_lowercase();

            let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.hmac_key)
                .expect("HMAC can take key of any size");

            mac.update(normalized.as_bytes());

            hex::encode(mac.finalize().into_bytes())
        }

        pub fn encrypt_opt(&self, plaintext: Option<&str>) -> Result<Option<String>, String> {
            match plaintext {
                Some(text) => Ok(Some(self.encrypt(text)?)),
                None => Ok(None),
            }
        }

        #[allow(dead_code)]
        pub fn decrypt_opt(&self, encrypted: Option<&str>) -> Result<Option<String>, String> {
            match encrypted {
                Some(enc) => Ok(Some(self.decrypt(enc)?)),
                None => Ok(None),
            }
        }

        pub fn blind_index_opt(&self, value: Option<&str>) -> Option<String> {
            value.map(|v| self.blind_index(v))
        }
    }
}

use encryption::EncryptionService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("migrate_pii_encryption=info".parse()?),
        )
        .init();

    tracing::info!("Starting PII encryption migration");

    // Load .env file
    dotenvy::dotenv().ok();

    // Get database URL
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Get encryption keys
    let encryption_key_b64 = env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY must be set");
    let hmac_key_b64 = env::var("ENCRYPTION_HMAC_KEY").expect("ENCRYPTION_HMAC_KEY must be set");

    // Decode keys
    let encryption_key = base64::engine::general_purpose::STANDARD
        .decode(&encryption_key_b64)
        .expect("ENCRYPTION_KEY must be valid base64");

    let hmac_key = base64::engine::general_purpose::STANDARD
        .decode(&hmac_key_b64)
        .expect("ENCRYPTION_HMAC_KEY must be valid base64");

    // Initialize encryption service
    let encryption_service = EncryptionService::new(&encryption_key, &hmac_key)
        .expect("Failed to create encryption service");

    tracing::info!("Encryption service initialized");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    tracing::info!("Database connection established");

    // Migrate expectations table
    migrate_expectations(&pool, &encryption_service).await?;

    // Migrate contributors table
    migrate_contributors(&pool, &encryption_service).await?;

    tracing::info!("PII encryption migration completed successfully");

    Ok(())
}

/// Migrate expectations table
async fn migrate_expectations(pool: &sqlx::PgPool, encryption: &EncryptionService) -> Result<()> {
    tracing::info!("Migrating expectations table...");

    // Fetch all records with plaintext email
    let records = sqlx::query!(
        r#"
        SELECT id, email
        FROM expectations
        WHERE email IS NOT NULL
          AND email_encrypted IS NULL
        "#
    )
    .fetch_all(pool)
    .await?;

    tracing::info!("Found {} expectations records to encrypt", records.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for record in records {
        if let Some(email) = record.email {
            match encrypt_and_update_expectation(pool, encryption, record.id, email).await {
                Ok(_) => success_count += 1,
                Err(e) => {
                    error_count += 1;
                    tracing::error!("Failed to encrypt expectation {}: {}", record.id, e);
                }
            }
        }
    }

    tracing::info!(
        "Expectations migration complete: {} success, {} errors",
        success_count,
        error_count
    );

    Ok(())
}

/// Encrypt and update a single expectation record
async fn encrypt_and_update_expectation(
    pool: &sqlx::PgPool,
    encryption: &EncryptionService,
    id: uuid::Uuid,
    email: String,
) -> Result<()> {
    // Encrypt email
    let email_encrypted = encryption.encrypt(&email).map_err(|e| anyhow::anyhow!(e))?;

    // Generate blind index
    let email_index = encryption.blind_index(&email);

    // Update record
    sqlx::query!(
        r#"
        UPDATE expectations
        SET email_encrypted = $1,
            email_index = $2
        WHERE id = $3
        "#,
        email_encrypted,
        email_index,
        id
    )
    .execute(pool)
    .await?;

    // Verify roundtrip
    let decrypted = encryption
        .decrypt(&email_encrypted)
        .map_err(|e| anyhow::anyhow!(e))?;
    if decrypted != email {
        anyhow::bail!("Roundtrip verification failed for expectation {}", id);
    }

    tracing::debug!("Encrypted expectation {}", id);

    Ok(())
}

/// Migrate contributors table
async fn migrate_contributors(pool: &sqlx::PgPool, encryption: &EncryptionService) -> Result<()> {
    tracing::info!("Migrating contributors table...");

    // Fetch all records with plaintext PII fields
    let records = sqlx::query!(
        r#"
        SELECT id, email, whatsapp, contact_email, contact_whatsapp
        FROM contributors
        WHERE (email IS NOT NULL AND email_encrypted IS NULL)
           OR (whatsapp IS NOT NULL AND whatsapp_encrypted IS NULL)
           OR (contact_email IS NOT NULL AND contact_email_encrypted IS NULL)
           OR (contact_whatsapp IS NOT NULL AND contact_whatsapp_encrypted IS NULL)
        "#
    )
    .fetch_all(pool)
    .await?;

    tracing::info!("Found {} contributors records to encrypt", records.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for record in records {
        match encrypt_and_update_contributor(
            pool,
            encryption,
            record.id,
            record.email.as_deref(),
            record.whatsapp.as_deref(),
            record.contact_email.as_deref(),
            record.contact_whatsapp.as_deref(),
        )
        .await
        {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                tracing::error!("Failed to encrypt contributor {}: {}", record.id, e);
            }
        }
    }

    tracing::info!(
        "Contributors migration complete: {} success, {} errors",
        success_count,
        error_count
    );

    Ok(())
}

/// Encrypt and update a single contributor record
#[allow(clippy::too_many_arguments)]
async fn encrypt_and_update_contributor(
    pool: &sqlx::PgPool,
    encryption: &EncryptionService,
    id: uuid::Uuid,
    email: Option<&str>,
    whatsapp: Option<&str>,
    contact_email: Option<&str>,
    contact_whatsapp: Option<&str>,
) -> Result<()> {
    // Encrypt fields
    let email_encrypted = encryption
        .encrypt_opt(email)
        .map_err(|e| anyhow::anyhow!(e))?;
    let whatsapp_encrypted = encryption
        .encrypt_opt(whatsapp)
        .map_err(|e| anyhow::anyhow!(e))?;
    let contact_email_encrypted = encryption
        .encrypt_opt(contact_email)
        .map_err(|e| anyhow::anyhow!(e))?;
    let contact_whatsapp_encrypted = encryption
        .encrypt_opt(contact_whatsapp)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Generate blind indexes
    let email_index = encryption.blind_index_opt(email);
    let whatsapp_index = encryption.blind_index_opt(whatsapp);
    let contact_email_index = encryption.blind_index_opt(contact_email);
    let contact_whatsapp_index = encryption.blind_index_opt(contact_whatsapp);

    // Update record
    sqlx::query!(
        r#"
        UPDATE contributors
        SET email_encrypted = $1,
            email_index = $2,
            whatsapp_encrypted = $3,
            whatsapp_index = $4,
            contact_email_encrypted = $5,
            contact_email_index = $6,
            contact_whatsapp_encrypted = $7,
            contact_whatsapp_index = $8
        WHERE id = $9
        "#,
        email_encrypted,
        email_index,
        whatsapp_encrypted,
        whatsapp_index,
        contact_email_encrypted,
        contact_email_index,
        contact_whatsapp_encrypted,
        contact_whatsapp_index,
        id
    )
    .execute(pool)
    .await?;

    // Verify roundtrips
    if let Some(encrypted) = &email_encrypted {
        let decrypted = encryption
            .decrypt(encrypted)
            .map_err(|e| anyhow::anyhow!(e))?;
        if Some(decrypted.as_str()) != email {
            anyhow::bail!("Roundtrip verification failed for contributor {} email", id);
        }
    }

    if let Some(encrypted) = &whatsapp_encrypted {
        let decrypted = encryption
            .decrypt(encrypted)
            .map_err(|e| anyhow::anyhow!(e))?;
        if Some(decrypted.as_str()) != whatsapp {
            anyhow::bail!(
                "Roundtrip verification failed for contributor {} whatsapp",
                id
            );
        }
    }

    if let Some(encrypted) = &contact_email_encrypted {
        let decrypted = encryption
            .decrypt(encrypted)
            .map_err(|e| anyhow::anyhow!(e))?;
        if Some(decrypted.as_str()) != contact_email {
            anyhow::bail!(
                "Roundtrip verification failed for contributor {} contact_email",
                id
            );
        }
    }

    if let Some(encrypted) = &contact_whatsapp_encrypted {
        let decrypted = encryption
            .decrypt(encrypted)
            .map_err(|e| anyhow::anyhow!(e))?;
        if Some(decrypted.as_str()) != contact_whatsapp {
            anyhow::bail!(
                "Roundtrip verification failed for contributor {} contact_whatsapp",
                id
            );
        }
    }

    tracing::debug!("Encrypted contributor {}", id);

    Ok(())
}
