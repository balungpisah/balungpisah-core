use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Service for encrypting and decrypting PII fields using AES-256-GCM
/// Also provides blind indexing for searchable fields using HMAC-SHA256
pub struct EncryptionService {
    cipher: Aes256Gcm,
    hmac_key: Vec<u8>,
}

impl EncryptionService {
    /// Create a new EncryptionService with the given keys
    ///
    /// # Arguments
    /// * `key` - 32-byte (256-bit) AES-256-GCM encryption key
    /// * `hmac_key` - 32-byte HMAC key for blind indexing
    ///
    /// # Returns
    /// * `Result<Self, String>` - EncryptionService or error message
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

    /// Encrypt a plaintext string using AES-256-GCM
    ///
    /// Returns base64-encoded string in format: "nonce:ciphertext"
    /// where both nonce and ciphertext are base64-encoded
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        // Generate random 12-byte (96-bit) nonce
        let nonce = Aes256Gcm::generate_nonce(&mut rand::rngs::OsRng);

        // Encrypt the plaintext
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;

        // Encode nonce and ciphertext as base64
        let nonce_b64 = base64::engine::general_purpose::STANDARD.encode(nonce);
        let ciphertext_b64 = base64::engine::general_purpose::STANDARD.encode(ciphertext);

        // Return in format "nonce:ciphertext"
        Ok(format!("{}:{}", nonce_b64, ciphertext_b64))
    }

    /// Decrypt an encrypted string using AES-256-GCM
    ///
    /// Expects input in format: "nonce:ciphertext" (both base64-encoded)
    pub fn decrypt(&self, encrypted: &str) -> Result<String, String> {
        // Split into nonce and ciphertext
        let parts: Vec<&str> = encrypted.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid encrypted format, expected 'nonce:ciphertext'".to_string());
        }

        // Decode base64
        let nonce_bytes = base64::engine::general_purpose::STANDARD
            .decode(parts[0])
            .map_err(|e| format!("Failed to decode nonce: {}", e))?;

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(parts[1])
            .map_err(|e| format!("Failed to decode ciphertext: {}", e))?;

        // Create nonce from bytes
        #[allow(deprecated)]
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| format!("Decryption failed: {}", e))?;

        // Convert to string
        String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    /// Generate a blind index for a value using HMAC-SHA256
    ///
    /// The value is normalized (lowercase, trimmed) for deterministic hashing
    /// Returns a 64-character hex string
    pub fn blind_index(&self, value: &str) -> String {
        // Normalize the value (lowercase, trim) for deterministic hashing
        let normalized = value.trim().to_lowercase();

        // Create HMAC instance
        let mut mac = <HmacSha256 as Mac>::new_from_slice(&self.hmac_key)
            .expect("HMAC can take key of any size");

        // Update with normalized value
        mac.update(normalized.as_bytes());

        // Get result and encode as hex
        hex::encode(mac.finalize().into_bytes())
    }

    /// Encrypt an optional string field
    ///
    /// Returns None if input is None, otherwise encrypts and returns Some(encrypted)
    pub fn encrypt_opt(&self, plaintext: Option<&str>) -> Result<Option<String>, String> {
        match plaintext {
            Some(text) => Ok(Some(self.encrypt(text)?)),
            None => Ok(None),
        }
    }

    /// Decrypt an optional string field
    ///
    /// Returns None if input is None, otherwise decrypts and returns Some(decrypted)
    pub fn decrypt_opt(&self, encrypted: Option<&str>) -> Result<Option<String>, String> {
        match encrypted {
            Some(enc) => Ok(Some(self.decrypt(enc)?)),
            None => Ok(None),
        }
    }

    /// Generate a blind index for an optional field
    ///
    /// Returns None if input is None, otherwise generates index and returns Some(index)
    pub fn blind_index_opt(&self, value: Option<&str>) -> Option<String> {
        value.map(|v| self.blind_index(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> EncryptionService {
        let key = vec![0u8; 32]; // 32-byte key
        let hmac_key = vec![1u8; 32]; // 32-byte HMAC key
        EncryptionService::new(&key, &hmac_key).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let service = create_test_service();
        let plaintext = "test@example.com";

        let encrypted = service.encrypt(plaintext).unwrap();
        assert!(
            encrypted.contains(':'),
            "Encrypted format should contain ':'"
        );

        let decrypted = service.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let service = create_test_service();
        let plaintext = "test@example.com";

        let encrypted1 = service.encrypt(plaintext).unwrap();
        let encrypted2 = service.encrypt(plaintext).unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        assert_eq!(service.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(service.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_invalid_format() {
        let service = create_test_service();

        let result = service.decrypt("invalid-no-colon");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid encrypted format"));
    }

    #[test]
    fn test_blind_index_determinism() {
        let service = create_test_service();
        let value = "test@example.com";

        let index1 = service.blind_index(value);
        let index2 = service.blind_index(value);

        assert_eq!(index1, index2);
        assert_eq!(index1.len(), 64); // SHA-256 hex is 64 characters
    }

    #[test]
    fn test_blind_index_normalization() {
        let service = create_test_service();

        let index1 = service.blind_index("Test@Example.com");
        let index2 = service.blind_index("test@example.com");
        let index3 = service.blind_index("  test@example.com  ");

        // All should produce the same index due to normalization
        assert_eq!(index1, index2);
        assert_eq!(index2, index3);
    }

    #[test]
    fn test_blind_index_different_values() {
        let service = create_test_service();

        let index1 = service.blind_index("test1@example.com");
        let index2 = service.blind_index("test2@example.com");

        assert_ne!(index1, index2);
    }

    #[test]
    fn test_encrypt_opt_with_none() {
        let service = create_test_service();

        let result = service.encrypt_opt(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_encrypt_opt_with_some() {
        let service = create_test_service();

        let result = service.encrypt_opt(Some("test@example.com")).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains(':'));
    }

    #[test]
    fn test_decrypt_opt_with_none() {
        let service = create_test_service();

        let result = service.decrypt_opt(None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_decrypt_opt_with_some() {
        let service = create_test_service();
        let plaintext = "test@example.com";

        let encrypted = service.encrypt(plaintext).unwrap();
        let result = service.decrypt_opt(Some(&encrypted)).unwrap();

        assert_eq!(result, Some(plaintext.to_string()));
    }

    #[test]
    fn test_blind_index_opt_with_none() {
        let service = create_test_service();

        let result = service.blind_index_opt(None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_blind_index_opt_with_some() {
        let service = create_test_service();

        let result = service.blind_index_opt(Some("test@example.com"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 64);
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = vec![0u8; 16]; // Too short
        let hmac_key = vec![1u8; 32];

        let result = EncryptionService::new(&short_key, &hmac_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hmac_key_length() {
        let key = vec![0u8; 32];
        let short_hmac_key = vec![1u8; 16]; // Too short

        let result = EncryptionService::new(&key, &short_hmac_key);
        assert!(result.is_err());
    }
}
