//! Encryption service for Resonance
//!
//! This module provides AES-256-GCM encryption for sensitive data like API keys.
//! The encryption key is derived from the JWT_SECRET using HKDF-SHA256.

// TODO: Remove this once EncryptionService is integrated with system settings
#![allow(dead_code)]

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use thiserror::Error;

/// Nonce size for AES-256-GCM (12 bytes / 96 bits)
const NONCE_SIZE: usize = 12;

/// Salt used for HKDF key derivation
/// This is a fixed, application-specific salt that provides domain separation
const HKDF_SALT: &[u8] = b"resonance-encryption-v1";

/// Info string for HKDF context
const HKDF_INFO: &[u8] = b"api-key-encryption";

/// Errors that can occur during encryption operations
#[derive(Error, Debug)]
pub enum EncryptionError {
    /// Failed to derive encryption key from JWT secret
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),

    /// Encryption operation failed
    #[error("encryption failed: {0}")]
    Encryption(String),

    /// Decryption operation failed
    #[error("decryption failed: {0}")]
    Decryption(String),

    /// Ciphertext is too short (missing nonce)
    #[error("ciphertext too short: expected at least {NONCE_SIZE} bytes for nonce")]
    CiphertextTooShort,

    /// Decrypted data is not valid UTF-8
    #[error("decrypted data is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

/// Service for encrypting and decrypting sensitive data using AES-256-GCM
///
/// The encryption key is derived from the JWT_SECRET using HKDF-SHA256,
/// ensuring the same key is used consistently across application restarts.
///
/// # Security Properties
///
/// - **Algorithm**: AES-256-GCM (authenticated encryption)
/// - **Key Derivation**: HKDF-SHA256 with application-specific salt
/// - **Nonce**: 12-byte random nonce, prepended to ciphertext
/// - **Authentication**: GCM provides built-in authentication tag
///
/// # Example
///
/// ```ignore
/// let service = EncryptionService::new("your-jwt-secret-at-least-32-chars");
/// let ciphertext = service.encrypt("my-api-key")?;
/// let plaintext = service.decrypt(&ciphertext)?;
/// assert_eq!(plaintext, "my-api-key");
/// ```
#[derive(Clone)]
pub struct EncryptionService {
    cipher: Aes256Gcm,
}

impl EncryptionService {
    /// Create a new EncryptionService with a key derived from the JWT secret
    ///
    /// # Arguments
    ///
    /// * `jwt_secret` - The JWT secret used to derive the encryption key via HKDF
    ///
    /// # Panics
    ///
    /// Panics if key derivation fails (which should not happen with valid input)
    pub fn new(jwt_secret: &str) -> Self {
        let key = Self::derive_key(jwt_secret).expect("Key derivation should not fail");
        let cipher = Aes256Gcm::new(&key.into());
        Self { cipher }
    }

    /// Try to create a new EncryptionService, returning an error on failure
    ///
    /// Use this method when you need to handle initialization errors gracefully.
    ///
    /// # Arguments
    ///
    /// * `jwt_secret` - The JWT secret used to derive the encryption key via HKDF
    ///
    /// # Errors
    ///
    /// Returns `EncryptionError::KeyDerivation` if key derivation fails
    pub fn try_new(jwt_secret: &str) -> Result<Self, EncryptionError> {
        let key = Self::derive_key(jwt_secret)?;
        let cipher = Aes256Gcm::new(&key.into());
        Ok(Self { cipher })
    }

    /// Derive a 256-bit encryption key from the JWT secret using HKDF-SHA256
    ///
    /// HKDF (HMAC-based Key Derivation Function) is used to derive a
    /// cryptographically strong key from the JWT secret. This provides:
    /// - Key stretching if the input has low entropy
    /// - Domain separation via the salt and info parameters
    /// - Consistent key derivation across restarts
    fn derive_key(jwt_secret: &str) -> Result<[u8; 32], EncryptionError> {
        let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), jwt_secret.as_bytes());
        let mut key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut key)
            .map_err(|e| EncryptionError::KeyDerivation(e.to_string()))?;
        Ok(key)
    }

    /// Encrypt plaintext data using AES-256-GCM
    ///
    /// A random 12-byte nonce is generated and prepended to the ciphertext.
    /// The resulting format is: `nonce (12 bytes) || ciphertext || auth_tag (16 bytes)`
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The string to encrypt
    ///
    /// # Returns
    ///
    /// A Vec<u8> containing the nonce followed by the authenticated ciphertext
    ///
    /// # Errors
    ///
    /// Returns `EncryptionError::Encryption` if the encryption operation fails
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, EncryptionError> {
        // Generate a random 12-byte nonce using OsRng
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        aes_gcm::aead::rand_core::RngCore::fill_bytes(&mut OsRng, &mut nonce_bytes);

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the plaintext
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend(ciphertext);

        Ok(result)
    }

    /// Decrypt ciphertext data using AES-256-GCM
    ///
    /// Expects the format: `nonce (12 bytes) || ciphertext || auth_tag (16 bytes)`
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted data (nonce + ciphertext + auth tag)
    ///
    /// # Returns
    ///
    /// The decrypted plaintext as a String
    ///
    /// # Errors
    ///
    /// - `EncryptionError::CiphertextTooShort` if input is less than 12 bytes
    /// - `EncryptionError::Decryption` if decryption or authentication fails
    /// - `EncryptionError::InvalidUtf8` if decrypted data is not valid UTF-8
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<String, EncryptionError> {
        // Ensure ciphertext is long enough to contain nonce
        if ciphertext.len() < NONCE_SIZE {
            return Err(EncryptionError::CiphertextTooShort);
        }

        // Split nonce and actual ciphertext
        let (nonce_bytes, encrypted_data) = ciphertext.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt the data
        let plaintext_bytes = self
            .cipher
            .decrypt(nonce, encrypted_data)
            .map_err(|e| EncryptionError::Decryption(e.to_string()))?;

        // Convert to string
        String::from_utf8(plaintext_bytes).map_err(EncryptionError::InvalidUtf8)
    }
}

impl std::fmt::Debug for EncryptionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Don't expose any internal state for security
        f.debug_struct("EncryptionService")
            .field("cipher", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test JWT secret that meets the minimum security requirements
    const TEST_JWT_SECRET: &str = "test-jwt-secret-that-is-at-least-32-characters-long";

    #[test]
    fn test_encryption_service_creation() {
        let service = EncryptionService::new(TEST_JWT_SECRET);
        // Just verify it doesn't panic
        let _ = format!("{:?}", service);
    }

    #[test]
    fn test_try_new_success() {
        let result = EncryptionService::try_new(TEST_JWT_SECRET);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "my-secret-api-key-12345";
        let ciphertext = service
            .encrypt(plaintext)
            .expect("encryption should succeed");
        let decrypted = service
            .decrypt(&ciphertext)
            .expect("decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_empty_string() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "";
        let ciphertext = service
            .encrypt(plaintext)
            .expect("encryption should succeed");
        let decrypted = service
            .decrypt(&ciphertext)
            .expect("decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_unicode() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "api-key-with-unicode-🔐🎵";
        let ciphertext = service
            .encrypt(plaintext)
            .expect("encryption should succeed");
        let decrypted = service
            .decrypt(&ciphertext)
            .expect("decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip_long_text() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "a".repeat(10000);
        let ciphertext = service
            .encrypt(&plaintext)
            .expect("encryption should succeed");
        let decrypted = service
            .decrypt(&ciphertext)
            .expect("decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_ciphertext_includes_nonce() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "test";
        let ciphertext = service
            .encrypt(plaintext)
            .expect("encryption should succeed");

        // Ciphertext should be at least nonce (12) + plaintext (4) + auth_tag (16) = 32 bytes
        assert!(ciphertext.len() >= NONCE_SIZE + 4 + 16);
    }

    #[test]
    fn test_different_encryptions_produce_different_ciphertexts() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "same-plaintext";
        let ciphertext1 = service
            .encrypt(plaintext)
            .expect("encryption should succeed");
        let ciphertext2 = service
            .encrypt(plaintext)
            .expect("encryption should succeed");

        // Due to random nonces, ciphertexts should be different
        assert_ne!(ciphertext1, ciphertext2);

        // But both should decrypt to the same plaintext
        let decrypted1 = service
            .decrypt(&ciphertext1)
            .expect("decryption should succeed");
        let decrypted2 = service
            .decrypt(&ciphertext2)
            .expect("decryption should succeed");
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn test_decrypt_ciphertext_too_short() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let short_ciphertext = vec![0u8; 5]; // Less than NONCE_SIZE
        let result = service.decrypt(&short_ciphertext);

        assert!(matches!(result, Err(EncryptionError::CiphertextTooShort)));
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        // Valid-length but invalid data (random bytes won't decrypt correctly)
        let invalid_ciphertext = vec![0u8; 50];
        let result = service.decrypt(&invalid_ciphertext);

        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }

    #[test]
    fn test_decrypt_tampered_ciphertext() {
        let service = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "my-api-key";
        let mut ciphertext = service
            .encrypt(plaintext)
            .expect("encryption should succeed");

        // Tamper with the ciphertext (after the nonce)
        if ciphertext.len() > NONCE_SIZE {
            ciphertext[NONCE_SIZE] ^= 0xFF;
        }

        // Decryption should fail due to authentication tag mismatch
        let result = service.decrypt(&ciphertext);
        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }

    #[test]
    fn test_different_secrets_produce_different_keys() {
        let service1 = EncryptionService::new("secret-one-that-is-at-least-32-characters");
        let service2 = EncryptionService::new("secret-two-that-is-at-least-32-characters");

        let plaintext = "test-data";
        let ciphertext = service1
            .encrypt(plaintext)
            .expect("encryption should succeed");

        // Decryption with different key should fail
        let result = service2.decrypt(&ciphertext);
        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }

    #[test]
    fn test_same_secret_produces_same_key() {
        // Create two services with the same secret
        let service1 = EncryptionService::new(TEST_JWT_SECRET);
        let service2 = EncryptionService::new(TEST_JWT_SECRET);

        let plaintext = "test-data";
        let ciphertext = service1
            .encrypt(plaintext)
            .expect("encryption should succeed");

        // Decryption with same key (different instance) should succeed
        let decrypted = service2
            .decrypt(&ciphertext)
            .expect("decryption should succeed");
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_debug_does_not_expose_key() {
        let service = EncryptionService::new(TEST_JWT_SECRET);
        let debug_output = format!("{:?}", service);

        // Debug output should not contain the secret
        assert!(!debug_output.contains(TEST_JWT_SECRET));
        assert!(debug_output.contains("REDACTED"));
    }

    #[test]
    fn test_key_derivation_consistency() {
        // HKDF should always derive the same key from the same input
        let key1 =
            EncryptionService::derive_key(TEST_JWT_SECRET).expect("key derivation should succeed");
        let key2 =
            EncryptionService::derive_key(TEST_JWT_SECRET).expect("key derivation should succeed");

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_derivation_different_inputs() {
        let key1 = EncryptionService::derive_key("secret-one-at-least-32-chars-long")
            .expect("key derivation should succeed");
        let key2 = EncryptionService::derive_key("secret-two-at-least-32-chars-long")
            .expect("key derivation should succeed");

        assert_ne!(key1, key2);
    }
}
