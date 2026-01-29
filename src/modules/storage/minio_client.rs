//! MinIO/S3-compatible storage client
//!
//! Provides file upload, download, and presigned URL generation
//! for MinIO or any S3-compatible storage service.
//!
//! Uses rust-s3 crate for lightweight S3 operations.

use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::{Client, Url};
use s3::creds::Credentials;
use s3::{Bucket, BucketConfiguration, Region};
use serde_json::json;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

use crate::core::config::MinIOConfig;
use crate::core::error::AppError;

type HmacSha256 = Hmac<Sha256>;

/// File visibility for uploaded files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileVisibility {
    /// Public files are accessible via direct URL
    Public,
    /// Private files require presigned URLs for access
    Private,
}

/// MinIO/S3-compatible storage client
#[allow(dead_code)]
pub struct MinIOClient {
    bucket: Box<Bucket>,
    region: Region,
    credentials: Credentials,
    presigned_url_expiry_secs: u32,
    endpoint: String,
    public_endpoint: String,
    public_prefix: String,
    private_prefix: String,
    /// Access key for AWS Signature v4 signing
    access_key: String,
    /// Secret key for AWS Signature v4 signing
    secret_key: String,
    /// Region name for AWS Signature v4 signing
    region_name: String,
    /// HTTP client for bucket policy operations
    http_client: Client,
}

#[allow(dead_code)]
impl MinIOClient {
    /// Create a new MinIO client from configuration
    ///
    /// This will:
    /// 1. Create the bucket if it doesn't exist
    /// 2. Set public read policy for the public prefix
    pub async fn new(config: MinIOConfig) -> Result<Self, AppError> {
        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )
        .map_err(|e| AppError::Internal(format!("Failed to create MinIO credentials: {}", e)))?;

        let region = Region::Custom {
            region: config.region.clone(),
            endpoint: config.endpoint.clone(),
        };

        let mut bucket = Bucket::new(&config.bucket, region.clone(), credentials.clone())
            .map_err(|e| AppError::Internal(format!("Failed to create MinIO bucket: {}", e)))?;

        // Use path-style URLs for MinIO (http://endpoint/bucket instead of http://bucket.endpoint)
        bucket.set_path_style();

        // Create HTTP client for bucket policy operations
        let http_client = Client::builder()
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let client = Self {
            bucket,
            region,
            credentials,
            presigned_url_expiry_secs: config.presigned_url_expiry_secs,
            endpoint: config.endpoint,
            public_endpoint: config.public_endpoint,
            public_prefix: config.public_prefix,
            private_prefix: config.private_prefix,
            access_key: config.access_key,
            secret_key: config.secret_key,
            region_name: config.region,
            http_client,
        };

        // Ensure bucket exists and set up policies
        client.ensure_bucket_exists().await?;
        client.set_public_read_policy().await?;

        info!(
            "MinIO client initialized for endpoint: {}, bucket: {}, public_prefix: {}, private_prefix: {}",
            client.endpoint, client.bucket.name(), client.public_prefix, client.private_prefix
        );

        Ok(client)
    }

    /// Ensure the bucket exists, create if not
    pub async fn ensure_bucket_exists(&self) -> Result<(), AppError> {
        // Try to create bucket - if it already exists, MinIO will return an error
        // which we can safely ignore
        match self.create_bucket().await {
            Ok(_) => {
                info!("Bucket '{}' created successfully", self.bucket.name());
                Ok(())
            }
            Err(e) => {
                let error_str = e.to_string();
                // Bucket already exists - this is fine
                if error_str.contains("BucketAlreadyOwnedByYou")
                    || error_str.contains("BucketAlreadyExists")
                    || error_str.contains("already own it")
                {
                    debug!("Bucket '{}' already exists", self.bucket.name());
                    Ok(())
                } else {
                    // Log warning but don't fail - bucket might exist with different error
                    warn!(
                        "Could not create bucket '{}': {}. Assuming it exists.",
                        self.bucket.name(),
                        e
                    );
                    Ok(())
                }
            }
        }
    }

    /// Create the bucket
    async fn create_bucket(&self) -> Result<(), AppError> {
        let bucket_config = BucketConfiguration::default();

        Bucket::create_with_path_style(
            &self.bucket.name(),
            self.region.clone(),
            self.credentials.clone(),
            bucket_config,
        )
        .await
        .map_err(|e| {
            AppError::Internal(format!(
                "Failed to create bucket '{}': {}",
                self.bucket.name(),
                e
            ))
        })?;

        Ok(())
    }

    /// Set public read policy for the public prefix
    ///
    /// This allows anonymous read access to files in the public prefix (e.g., `public/*`).
    /// Files in the private prefix remain inaccessible without authentication.
    async fn set_public_read_policy(&self) -> Result<(), AppError> {
        let bucket_name = self.bucket.name();
        let public_prefix = &self.public_prefix;

        // S3 bucket policy for public read access on public prefix
        let policy = json!({
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Allow",
                    "Principal": {"AWS": "*"},
                    "Action": ["s3:GetObject"],
                    "Resource": [format!("arn:aws:s3:::{bucket_name}/{public_prefix}/*")]
                }
            ]
        });

        let policy_str = policy.to_string();

        // Use AWS Signature v4 to sign the request
        match self
            .put_bucket_policy_with_sigv4(&bucket_name, &policy_str)
            .await
        {
            Ok(_) => {
                info!(
                    "Set public read policy for {}/{}/*",
                    bucket_name, public_prefix
                );
                Ok(())
            }
            Err(e) => {
                // Log warning but don't fail startup - policy can be set manually
                warn!(
                    "Failed to set bucket policy for '{}': {}. \
                    You may need to set the policy manually using: \
                    mc anonymous set download minio/{}/{}",
                    bucket_name, e, bucket_name, public_prefix
                );
                Ok(())
            }
        }
    }

    /// Put bucket policy using AWS Signature v4
    async fn put_bucket_policy_with_sigv4(
        &self,
        bucket_name: &str,
        policy: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        // Parse endpoint to get host
        let endpoint_url = Url::parse(&self.endpoint)
            .map_err(|e| AppError::Internal(format!("Invalid endpoint URL: {}", e)))?;
        let host = endpoint_url
            .host_str()
            .ok_or_else(|| AppError::Internal("Endpoint URL has no host".to_string()))?;
        let port = endpoint_url.port();
        let host_header = match port {
            Some(p) => format!("{}:{}", host, p),
            None => host.to_string(),
        };

        // Build the URL for PUT bucket policy
        let url = format!("{}/{}?policy", self.endpoint, bucket_name);

        // Calculate payload hash
        let payload_hash = hex::encode(Sha256::digest(policy.as_bytes()));

        // Create canonical request
        let canonical_uri = format!("/{}", bucket_name);
        let canonical_querystring = "policy=";
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host_header, payload_hash, amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";

        let canonical_request = format!(
            "PUT\n{}\n{}\n{}\n{}\n{}",
            canonical_uri, canonical_querystring, canonical_headers, signed_headers, payload_hash
        );

        // Create string to sign
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, self.region_name);
        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm, amz_date, credential_scope, canonical_request_hash
        );

        // Calculate signature
        let signature = self.calculate_signature(&date_stamp, &string_to_sign)?;

        // Create authorization header
        let authorization_header = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm, self.access_key, credential_scope, signed_headers, signature
        );

        // Make the request
        let response = self
            .http_client
            .put(&url)
            .header("Host", &host_header)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload_hash)
            .header("Authorization", &authorization_header)
            .header("Content-Type", "application/json")
            .body(policy.to_string())
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send policy request: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(AppError::Internal(format!(
                "Failed to set bucket policy: {} - {}",
                status, body
            )))
        }
    }

    /// Calculate AWS Signature v4 signature
    fn calculate_signature(
        &self,
        date_stamp: &str,
        string_to_sign: &str,
    ) -> Result<String, AppError> {
        // Step 1: Create signing key
        let k_date = Self::hmac_sha256(
            format!("AWS4{}", self.secret_key).as_bytes(),
            date_stamp.as_bytes(),
        )?;
        let k_region = Self::hmac_sha256(&k_date, self.region_name.as_bytes())?;
        let k_service = Self::hmac_sha256(&k_region, b"s3")?;
        let k_signing = Self::hmac_sha256(&k_service, b"aws4_request")?;

        // Step 2: Calculate signature
        let signature = Self::hmac_sha256(&k_signing, string_to_sign.as_bytes())?;
        Ok(hex::encode(signature))
    }

    /// HMAC-SHA256 helper
    fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, AppError> {
        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|e| AppError::Internal(format!("HMAC key error: {}", e)))?;
        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    /// Get the prefix for the given visibility
    pub fn get_prefix(&self, visibility: FileVisibility) -> &str {
        match visibility {
            FileVisibility::Public => &self.public_prefix,
            FileVisibility::Private => &self.private_prefix,
        }
    }

    /// Generate a file key with the appropriate prefix based on visibility
    ///
    /// # Arguments
    /// * `visibility` - Whether the file should be public or private
    /// * `path` - The path within the visibility prefix (e.g., "user123/file.pdf")
    ///
    /// # Returns
    /// The full file key (e.g., "public/user123/file.pdf" or "private/user123/file.pdf")
    pub fn generate_key(&self, visibility: FileVisibility, path: &str) -> String {
        let prefix = self.get_prefix(visibility);
        format!("{}/{}", prefix, path)
    }

    /// Upload a file to the storage
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the bucket
    /// * `data` - The file content as bytes
    /// * `content_type` - The MIME type of the file
    ///
    /// # Returns
    /// The object key (path) of the uploaded file
    pub async fn upload(
        &self,
        key: &str,
        data: Vec<u8>,
        content_type: &str,
    ) -> Result<String, AppError> {
        self.bucket
            .put_object_with_content_type(key, &data, content_type)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to upload file '{}': {}", key, e)))?;

        debug!("Uploaded file '{}' to bucket '{}'", key, self.bucket.name());
        Ok(key.to_string())
    }

    /// Generate a presigned URL for downloading a file
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the bucket
    ///
    /// # Returns
    /// A presigned URL that allows temporary access to the file
    pub async fn get_presigned_url(&self, key: &str) -> Result<String, AppError> {
        let url = self
            .bucket
            .presign_get(key, self.presigned_url_expiry_secs, None)
            .await
            .map_err(|e| {
                AppError::Internal(format!(
                    "Failed to generate presigned URL for '{}': {}",
                    key, e
                ))
            })?;

        Ok(url)
    }

    /// Download a file from the storage
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the bucket
    ///
    /// # Returns
    /// The file content as bytes
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let response =
            self.bucket.get_object(key).await.map_err(|e| {
                AppError::Internal(format!("Failed to download file '{}': {}", key, e))
            })?;

        debug!(
            "Downloaded file '{}' from bucket '{}'",
            key,
            self.bucket.name()
        );
        Ok(response.to_vec())
    }

    /// Delete a file from the storage
    ///
    /// # Arguments
    /// * `key` - The object key (path) to delete
    pub async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to delete file '{}': {}", key, e)))?;

        debug!(
            "Deleted file '{}' from bucket '{}'",
            key,
            self.bucket.name()
        );
        Ok(())
    }

    /// Check if a file exists in the storage
    ///
    /// # Arguments
    /// * `key` - The object key (path) to check
    ///
    /// # Returns
    /// `true` if the file exists, `false` otherwise
    pub async fn exists(&self, key: &str) -> Result<bool, AppError> {
        match self.bucket.head_object(key).await {
            Ok(_) => Ok(true),
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("404") || error_str.contains("NoSuchKey") {
                    Ok(false)
                } else {
                    Err(AppError::Internal(format!(
                        "Failed to check if file '{}' exists: {}",
                        key, e
                    )))
                }
            }
        }
    }

    /// Get the presigned URL expiry time in seconds
    pub fn presigned_url_expiry_secs(&self) -> u32 {
        self.presigned_url_expiry_secs
    }

    /// Get the bucket name
    pub fn bucket_name(&self) -> String {
        self.bucket.name()
    }

    /// Get the public prefix
    pub fn public_prefix(&self) -> &str {
        &self.public_prefix
    }

    /// Get the private prefix
    pub fn private_prefix(&self) -> &str {
        &self.private_prefix
    }

    /// Get the URL for a file based on its visibility
    ///
    /// For public files, returns a direct public URL using the public endpoint.
    /// For private files, this returns the internal URL (use get_presigned_url for access).
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the bucket
    ///
    /// # Returns
    /// The URL for the file
    pub fn get_file_url(&self, key: &str) -> String {
        if key.starts_with(&self.public_prefix) {
            // Use public endpoint for public files
            format!("{}/{}/{}", self.public_endpoint, self.bucket.name(), key)
        } else {
            // Use internal endpoint for private files
            format!("{}/{}/{}", self.endpoint, self.bucket.name(), key)
        }
    }

    /// Get the public URL for a file (legacy method, uses public endpoint)
    ///
    /// This generates a direct URL to the file (requires bucket to be public or configured for public access)
    ///
    /// # Arguments
    /// * `key` - The object key (path) in the bucket
    ///
    /// # Returns
    /// The public URL for the file
    pub fn get_public_url(&self, key: &str) -> String {
        format!("{}/{}/{}", self.public_endpoint, self.bucket.name(), key)
    }

    /// Extract file key from a URL
    ///
    /// # Arguments
    /// * `url` - The URL of the file (can be public or internal endpoint)
    ///
    /// # Returns
    /// The file key if the URL matches this client's endpoints and bucket, None otherwise
    pub fn extract_key_from_url(&self, url: &str) -> Option<String> {
        // Try public endpoint first
        let public_prefix = format!("{}/{}/", self.public_endpoint, self.bucket.name());
        if url.starts_with(&public_prefix) {
            return Some(url[public_prefix.len()..].to_string());
        }

        // Try internal endpoint
        let internal_prefix = format!("{}/{}/", self.endpoint, self.bucket.name());
        if url.starts_with(&internal_prefix) {
            return Some(url[internal_prefix.len()..].to_string());
        }

        None
    }

    /// Check if a file key is for a public file
    pub fn is_public_key(&self, key: &str) -> bool {
        key.starts_with(&format!("{}/", self.public_prefix))
    }

    /// Check if a file key is for a private file
    pub fn is_private_key(&self, key: &str) -> bool {
        key.starts_with(&format!("{}/", self.private_prefix))
    }
}
