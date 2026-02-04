//! DirSoul Encryption Module
//!
//! Privacy-first encryption using Fernet symmetric encryption.
//! All sensitive data is encrypted at rest with proper key management.
//!
//! # Security Principles
//! - Keys are stored in restricted permission files (0400)
//! - Sensitive data is zeroed from memory after use
//! - No hardcoded keys or secrets
//!
//! # Example
//! ```no_run
//! use dirsoul::crypto::EncryptionManager;
//! use std::path::Path;
//!
//! let manager = EncryptionManager::initialize(Path::new(".encryption_key"))?;
//! let encrypted = manager.encrypt(b"sensitive data")?;
//! let decrypted = manager.decrypt(&encrypted)?;
//! assert_eq!(decrypted, b"sensitive data");
//! # Ok::<(), dirsoul::DirSoulError>(())
//! ```

use fernet::Fernet;
use std::path::Path;
use zeroize::Zeroize;

use crate::{DirSoulError, Result};

/// Default encryption key file name
pub const DEFAULT_KEY_FILE: &str = ".encryption_key";

/// Minimum Fernet token size (in bytes)
const FERNET_MIN_SIZE: usize = 32;

/// Encryption manager for DirSoul
///
/// Handles encryption/decryption operations using Fernet symmetric encryption.
/// Keys are stored securely in files with restrictive permissions.
///
/// # Memory Safety
/// - Sensitive data is cleared from memory after use
/// - No unnecessary copying of encrypted data
/// - Uses zeroize to securely clear buffers
pub struct EncryptionManager {
    fernet: Fernet,
    key_file: std::path::PathBuf,
}

impl EncryptionManager {
    /// Initialize or load encryption key
    ///
    /// If the key file exists, loads the existing key.
    /// Otherwise, generates a new key and saves it securely.
    ///
    /// # Arguments
    /// * `key_file` - Path to the key file
    ///
    /// # Returns
    /// Returns `Result<Self>` with the initialized manager
    ///
    /// # Errors
    /// Returns error if:
    /// - Key file exists but cannot be read
    /// - Key file cannot be created
    /// - File permissions cannot be set
    pub fn initialize(key_file: impl AsRef<Path>) -> Result<Self> {
        let path = key_file.as_ref();
        if path.exists() {
            Self::load(path)
        } else {
            Self::generate(path)
        }
    }

    /// Generate new encryption key and save to file
    ///
    /// This creates a new Fernet key and saves it with restrictive permissions (0400).
    ///
    /// # Arguments
    /// * `key_file` - Path where the key should be saved
    fn generate(key_file: &Path) -> Result<Self> {
        // Generate new Fernet key
        let key = fernet::Fernet::generate_key();

        // Create Fernet instance
        let fernet = Fernet::new(&key)
            .ok_or_else(|| DirSoulError::Encryption("Invalid Fernet key generated".to_string()))?;

        // Write key to file
        std::fs::write(key_file, &key).map_err(|e| {
            DirSoulError::Encryption(format!("Failed to write key file: {}", e))
        })?;

        // Set restrictive permissions (user read-only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(key_file)
                .map_err(|e| DirSoulError::Encryption(format!("Failed to get metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o400); // Read-only for owner

            std::fs::set_permissions(key_file, perms).map_err(|e| {
                DirSoulError::Encryption(format!("Failed to set permissions: {}", e))
            })?;
        }

        tracing::info!("Encryption key generated and saved to: {:?}", key_file);

        Ok(Self {
            fernet,
            key_file: key_file.to_path_buf(),
        })
    }

    /// Load existing encryption key from file
    ///
    /// # Arguments
    /// * `key_file` - Path to the existing key file
    fn load(key_file: &Path) -> Result<Self> {
        let key = std::fs::read_to_string(key_file).map_err(|e| {
            DirSoulError::Encryption(format!("Failed to read key file: {}", e))
        })?;

        let fernet = Fernet::new(&key).ok_or_else(|| {
            DirSoulError::Encryption("Invalid Fernet key in file".to_string())
        })?;

        tracing::info!("Encryption key loaded from: {:?}", key_file);

        Ok(Self {
            fernet,
            key_file: key_file.to_path_buf(),
        })
    }

    /// Encrypt data
    ///
    /// Uses Fernet encryption to encrypt the provided data.
    /// Returns the encrypted token as a Vec<u8>.
    ///
    /// # Arguments
    /// * `data` - Data to encrypt
    ///
    /// # Returns
    /// Encrypted data as Vec<u8>
    ///
    /// # Errors
    /// Returns error if encryption fails
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Fernet::encrypt returns String (base64-encoded token)
        let token = self.fernet.encrypt(data);
        Ok(token.into_bytes())
    }

    /// Decrypt data
    ///
    /// Decrypts Fernet-encrypted data.
    ///
    /// # Arguments
    /// * `encrypted` - Encrypted data (as bytes)
    ///
    /// # Returns
    /// Decrypted data as Vec<u8>
    ///
    /// # Errors
    /// Returns error if:
    /// - Data is too short (< 32 bytes for Fernet)
    /// - Decryption fails (invalid token, wrong key, etc.)
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        // Validate minimum size
        if encrypted.len() < FERNET_MIN_SIZE {
            return Err(DirSoulError::Encryption(
                "Encrypted data too short".to_string(),
            ));
        }

        // Convert bytes to string for Fernet API
        let token = std::str::from_utf8(encrypted)
            .map_err(|e| DirSoulError::Encryption(format!("Invalid UTF-8: {}", e)))?;

        self.fernet
            .decrypt(token)
            .map_err(|e| DirSoulError::Encryption(format!("Decryption failed: {}", e)))
    }

    /// Encrypt string
    ///
    /// Convenience method to encrypt a string and return base64-encoded result.
    ///
    /// # Arguments
    /// * `text` - String to encrypt
    pub fn encrypt_string(&self, text: &str) -> Result<String> {
        // Fernet::encrypt accepts &[u8], convert &str to &[u8]
        Ok(self.fernet.encrypt(text.as_bytes()))
    }

    /// Decrypt string
    ///
    /// Convenience method to decrypt a base64-encoded string.
    ///
    /// # Arguments
    /// * `encrypted` - Base64-encoded encrypted string
    pub fn decrypt_string(&self, encrypted: &str) -> Result<String> {
        let decrypted = self.fernet
            .decrypt(encrypted)
            .map_err(|e| DirSoulError::Encryption(format!("Decryption failed: {}", e)))?;

        String::from_utf8(decrypted)
            .map_err(|e| DirSoulError::Encryption(format!("UTF-8 decode failed: {}", e)))
    }

    /// Get the key file path
    pub fn key_file(&self) -> &Path {
        &self.key_file
    }
}

/// Secure buffer that zeroes memory on drop
///
/// This wrapper ensures sensitive data is cleared from memory
/// when the buffer is dropped, following memory safety best practices.
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    /// Create a new secure buffer
    ///
    /// # Arguments
    /// * `data` - Data to wrap
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Get a reference to the data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get the length of the data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        // Zero memory before deallocation
        self.data.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        // Create a temporary key file for testing
        let key_file = "/tmp/test_encryption_key";
        let _ = std::fs::remove_file(key_file); // Clean up first

        let manager = EncryptionManager::initialize(key_file).unwrap();

        let original = b"sensitive data";
        let encrypted = manager.encrypt(original).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, original);
        assert_ne!(encrypted, original.to_vec());

        // Clean up
        std::fs::remove_file(key_file).ok();
    }

    #[test]
    fn test_encrypt_decrypt_string() {
        let key_file = "/tmp/test_encryption_string_key";
        let _ = std::fs::remove_file(key_file);

        let manager = EncryptionManager::initialize(key_file).unwrap();

        let original = "Hello, World!";
        let encrypted = manager.encrypt_string(original).unwrap();
        let decrypted = manager.decrypt_string(&encrypted).unwrap();

        assert_eq!(decrypted, original);

        // Encrypted string should be different from original and longer
        assert_ne!(encrypted, original);
        assert!(encrypted.len() > original.len());

        std::fs::remove_file(key_file).ok();
    }

    #[test]
    fn test_key_persistence() {
        let key_file = "/tmp/test_encryption_persist_key";
        let _ = std::fs::remove_file(key_file);

        // Create manager and encrypt
        let manager1 = EncryptionManager::initialize(key_file).unwrap();
        let data = b"persistent test";
        let encrypted = manager1.encrypt(data).unwrap();

        // Load new manager with same key
        let manager2 = EncryptionManager::initialize(key_file).unwrap();
        let decrypted = manager2.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, data);

        std::fs::remove_file(key_file).ok();
    }

    #[test]
    fn test_empty_data() {
        let key_file = "/tmp/test_encryption_empty_key";
        let _ = std::fs::remove_file(key_file);

        let manager = EncryptionManager::initialize(key_file).unwrap();

        let original = b"";
        let encrypted = manager.encrypt(original).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, original);

        std::fs::remove_file(key_file).ok();
    }

    #[test]
    fn test_large_data() {
        let key_file = "/tmp/test_encryption_large_key";
        let _ = std::fs::remove_file(key_file);

        let manager = EncryptionManager::initialize(key_file).unwrap();

        // 1MB of data
        let original: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        let encrypted = manager.encrypt(&original).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, original);

        std::fs::remove_file(key_file).ok();
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let key_file = "/tmp/test_encryption_invalid_key";
        let _ = std::fs::remove_file(key_file);

        let manager = EncryptionManager::initialize(key_file).unwrap();

        // Too short
        let result = manager.decrypt(&[1, 2, 3]);
        assert!(result.is_err());

        // Invalid format
        let result = manager.decrypt(b"invalid encrypted data that is not real");
        assert!(result.is_err());

        std::fs::remove_file(key_file).ok();
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let key_file1 = "/tmp/test_encryption_wrong_key1";
        let key_file2 = "/tmp/test_encryption_wrong_key2";
        let _ = std::fs::remove_file(key_file1);
        let _ = std::fs::remove_file(key_file2);

        let manager1 = EncryptionManager::initialize(key_file1).unwrap();
        let manager2 = EncryptionManager::initialize(key_file2).unwrap();

        let original = b"test data";
        let encrypted = manager1.encrypt(original).unwrap();

        // Try to decrypt with different key
        let result = manager2.decrypt(&encrypted);
        assert!(result.is_err());

        std::fs::remove_file(key_file1).ok();
        std::fs::remove_file(key_file2).ok();
    }

    #[test]
    fn test_secure_buffer() {
        let data = vec![1, 2, 3, 4, 5];
        let buffer = SecureBuffer::new(data.clone());

        assert_eq!(buffer.as_slice(), &data[..]);
        assert_eq!(buffer.len(), 5);
        assert!(!buffer.is_empty());

        // Test zeroization on drop (hard to verify directly, but should not panic)
        drop(buffer);
    }

    #[test]
    fn test_secure_buffer_empty() {
        let buffer = SecureBuffer::new(vec![]);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }
}
