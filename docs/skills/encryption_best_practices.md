# Skill: Encryption Best Practices

> **Purpose**: Reinforce privacy-first design, ensuring end-to-end encryption and proper key management for all data storage modules.

---

## Core Encryption Architecture

### Fernet Rules

```rust
use cryptography::Fernet;

/// Encryption wrapper for DirSoul
pub struct EncryptionManager {
    fernet: Fernet,
    key_file: PathBuf,
}

impl EncryptionManager {
    /// Initialize or load encryption key
    pub fn initialize(key_file: &Path) -> Result<Self> {
        if key_file.exists() {
            // Load existing key
            Self::load(key_file)
        } else {
            // Generate new key
            Self::generate(key_file)
        }
    }

    /// Generate new encryption key
    fn generate(key_file: &Path) -> Result<Self> {
        let key = Fernet::generate_key();
        let fernet = Fernet::new(&key)?;

        // Store key securely (restrictive permissions)
        std::fs::write(key_file, &key)?;

        // Set file permissions (user read-only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(key_file)?.permissions();
            perms.set_mode(0o400);  // Read-only for owner
            std::fs::set_permissions(key_file, perms)?;
        }

        Ok(Self {
            fernet,
            key_file: key_file.to_path_buf(),
        })
    }

    /// Load existing key
    fn load(key_file: &Path) -> Result<Self> {
        let key = std::fs::read_to_string(key_file)?;
        let fernet = Fernet::new(&key)?;

        Ok(Self {
            fernet,
            key_file: key_file.to_path_buf(),
        })
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(self.fernet.encrypt(data)?)
    }

    /// Decrypt data
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        Ok(self.fernet.decrypt(encrypted)?)
    }

    /// Encrypt string
    pub fn encrypt_string(&self, text: &str) -> Result<String> {
        let encrypted = self.fernet.encrypt(text.as_bytes())?;
        Ok(base64::encode(&encrypted))
    }

    /// Decrypt string
    pub fn decrypt_string(&self, encrypted: &str) -> Result<String> {
        let decoded = base64::decode(encrypted)?;
        let decrypted = self.fernet.decrypt(&decoded)?;
        Ok(String::from_utf8(decrypted)?)
    }
}
```

---

## BYTEA Field Usage

### PostgreSQL Encryption Pattern

```sql
-- Schema with encrypted content
CREATE TABLE raw_memories (
    memory_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    content_type TEXT NOT NULL,

    -- Either plaintext OR encrypted, never both
    content TEXT,
    encrypted BYTEA,

    metadata JSONB DEFAULT '{}',
    embedding VECTOR(768),

    -- Ensure mutual exclusivity
    CONSTRAINT chk_encrypted_content CHECK (
        (content IS NOT NULL AND encrypted IS NULL) OR
        (content IS NULL AND encrypted IS NOT NULL)
    )
);
```

### Rust Integration

```rust
use diesel::sql_types::Bytea;

#[derive(Debug, Queryable, Identifiable)]
#[diesel(table_name = raw_memories)]
pub struct RawMemory {
    pub id: Uuid,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub content_type: String,
    pub content: Option<String>,
    pub encrypted: Option<Vec<u8>>,
    pub metadata: serde_json::Value,
}

impl RawMemory {
    /// Create encrypted memory
    pub fn new_encrypted(
        user_id: String,
        content_type: String,
        content: &str,
        encryption: &EncryptionManager
    ) -> Result<Self> {
        let encrypted = encryption.encrypt_string(content)?;

        Ok(Self {
            id: Uuid::new_v4(),
            user_id,
            created_at: Utc::now(),
            content_type,
            content: None,
            encrypted: Some(encrypted.into_bytes()),
            metadata: json!({}),
        })
    }

    /// Create plaintext memory (for testing/debugging)
    #[cfg(debug_assertions)]
    pub fn new_plaintext(
        user_id: String,
        content_type: String,
        content: String
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            created_at: Utc::now(),
            content_type,
            content: Some(content),
            encrypted: None,
            metadata: json!({}),
        }
    }

    /// Get decrypted content
    pub fn get_content(&self, encryption: &EncryptionManager) -> Result<String> {
        if let Some(ref text) = self.content {
            Ok(text.clone())
        } else if let Some(ref enc) = self.encrypted {
            let encrypted_str = String::from_utf8(enc.clone())?;
            encryption.decrypt_string(&encrypted_str)
        } else {
            Err(Error::NoContent)
        }
    }

    /// Check if memory is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encrypted.is_some()
    }
}
```

---

## Key Management

### Key Rotation Strategy

```rust
/// Key rotation manager
pub struct KeyRotationManager {
    current: EncryptionManager,
    previous: Vec<EncryptionManager>,
    rotation_interval: Duration,  // Default: 1 year
}

impl KeyRotationManager {
    /// Check if rotation is needed
    pub fn needs_rotation(&self) -> bool {
        let key_age = self.get_key_age();
        key_age > self.rotation_interval
    }

    /// Rotate encryption key
    pub async fn rotate(&mut self) -> Result<()> {
        // Create new key
        let new_key_path = self.current.key_file.with_extension("new");
        let new_manager = EncryptionManager::generate(&new_key_path)?;

        // Push current to previous
        self.previous.push(std::mem::replace(&mut self.current, new_manager));

        // Re-encrypt all data with new key
        self.reencrypt_all_data().await?;

        // Replace old key file
        std::fs::remove_file(self.previous.last().unwrap().key_file)?;

        Ok(())
    }

    /// Re-encrypt all existing data
    async fn reencrypt_all_data(&self) -> Result<()> {
        // This would be a long-running operation
        // For each encrypted memory:
        // 1. Decrypt with old key
        // 2. Encrypt with new key
        // 3. Update database

        // Pseudocode:
        // for memory in get_all_encrypted_memories().await? {
        //     let content = memory.get_content(&old_key)?;
        //     let new_encrypted = new_key.encrypt_string(&content)?;
        //     update_memory_encrypted(memory.id, &new_encrypted).await?;
        // }

        Ok(())
    }

    fn get_key_age(&self) -> Duration {
        // Get file creation time
        let metadata = std::fs::metadata(&self.current.key_file).ok()?;
        let modified = metadata.modified().ok()?;
        let now = std::time::SystemTime::now();
        now.duration_since(modified).ok()
    }
}
```

### Key Backup Rules

```rust
/// Key backup utility
pub struct KeyBackup {
    encryption: EncryptionManager,
}

impl KeyBackup {
    /// Export encrypted key backup
    pub fn export_backup(&self, password: &str) -> Result<Vec<u8>> {
        // Read the key file
        let key_data = std::fs::read(&self.encryption.key_file)?;

        // Encrypt backup with user-provided password
        let backup_key = self.derive_backup_key(password)?;
        let backup_encrypted = self.encrypt_with_key(&key_data, &backup_key)?;

        Ok(backup_encrypted)
    }

    /// Import key from backup
    pub fn import_backup(
        encrypted_backup: &[u8],
        password: &str,
        destination: &Path
    ) -> Result<()> {
        let backup_key = Self::derive_backup_key(password)?;
        let key_data = Self::decrypt_with_key(encrypted_backup, &backup_key)?;

        std::fs::write(destination, key_data)?;

        Ok(())
    }

    /// Derive encryption key from password
    fn derive_backup_key(password: &str) -> Result<Vec<u8>> {
        // Use Argon2 for key derivation
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::{SaltString, rand_core::OsRng};

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        // This is simplified - use proper key derivation in production
        Ok(password.as_bytes().to_vec())
    }
}
```

---

## Data Export/Import (GDPR Compliance)

### Encrypted Export

```rust
/// Data export with encryption
pub struct DataExporter {
    encryption: EncryptionManager,
}

impl DataExporter {
    /// Export all user data as encrypted package
    pub async fn export_user_data(
        &self,
        user_id: &str,
        include_raw: bool
    ) -> Result<DataExport> {
        let mut export = DataExport {
            user_id: user_id.to_string(),
            exported_at: Utc::now(),
            raw_memories: vec![],
            events: vec![],
            entities: vec![],
            views: vec![],
        };

        // Export raw memories (if permission granted)
        if include_raw {
            export.raw_memories = self.get_raw_memories(user_id).await?;
            for memory in &mut export.raw_memories {
                // Keep encrypted, but include in export
                // User can decrypt with their key
            }
        }

        // Export events (not encrypted, but structured)
        export.events = self.get_events(user_id).await?;

        // Export entities
        export.entities = self.get_entities(user_id).await?;

        // Export views
        export.views = self.get_views(user_id).await?;

        Ok(export)
    }

    /// Import data from encrypted backup
    pub async fn import_user_data(
        &self,
        encrypted_export: &[u8],
        password: &str
    ) -> Result<()> {
        // Decrypt export
        let export_data = self.decrypt_export(encrypted_export, password)?;

        // Validate data integrity
        self.validate_export(&export_data)?;

        // Import into database (with conflict resolution)
        self.import_to_database(export_data).await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataExport {
    pub user_id: String,
    pub exported_at: DateTime<Utc>,
    pub raw_memories: Vec<RawMemory>,
    pub events: Vec<Event>,
    pub entities: Vec<Entity>,
    pub views: Vec<DerivedView>,
}
```

---

## Security Checklist

### Before Deployment

- [ ] **Key storage**: Key file has restrictive permissions (0400)
- [ ] **Key backup**: User has been prompted to backup encryption key
- [ ] **No plaintext**: No sensitive data stored in plaintext in production
- [ ] **Key rotation**: Rotation schedule configured (recommended: yearly)
- [ ] **Audit logging**: All decryption attempts logged
- [ ] **Memory safety**: Sensitive data cleared from memory after use
- [ ] **Transport encryption**: Database connection uses TLS/SSL
- [ ] **Backup encryption**: Data backups are encrypted

---

## Memory Safety for Encrypted Data

### Clearing Sensitive Data

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure buffer that zeroes memory on drop
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        self.data.zeroize();  // Zero memory before deallocation
    }
}

/// Usage example
pub fn process_sensitive_data(encrypted: &[u8]) -> Result<()> {
    let decrypted = decrypt_data(encrypted)?;

    // Wrap in secure buffer
    let secure_buf = SecureBuffer::new(decrypted);

    // Process...
    let result = analyze_data(secure_buf.as_slice());

    // Buffer automatically zeroed when dropped
    Ok(result)
}
```

---

## Common Pitfalls

### Mistakes to Avoid

```rust
// ❌ MISTAKE 1: Storing key in code
const ENCRYPTION_KEY: &[u8] = b"hardcoded_key";  // NEVER DO THIS

// ✅ CORRECT: Load from secure file
let key = std::fs::read("encryption.key")?;

// ❌ MISTAKE 2: Logging sensitive data
debug!("Decrypted content: {}", decrypted);  // Logs are persistent

// ✅ CORRECT: Sanitize logs
debug!("Decrypted content length: {}", decrypted.len());

// ❌ MISTAKE 3: Storing both plaintext and encrypted
struct BadMemory {
    content: String,      // Plaintext!
    encrypted: Vec<u8>,   // AND encrypted?!
}

// ✅ CORRECT: Mutually exclusive
struct GoodMemory {
    content: Option<String>,
    encrypted: Option<Vec<u8>>,
}

// ❌ MISTAKE 4: Not handling encryption failures
let content = encrypt(data).unwrap();  // Could panic!

// ✅ CORRECT: Proper error handling
let encrypted = encrypt(data).map_err(|e| {
    error!("Encryption failed: {}", e);
    Error::EncryptionFailed
})?;
```

---

## Recommended Combinations

Use this skill together with:
- **RustMemorySafety**: For secure memory handling
- **PostgresSchemaDesign**: For BYTEA storage patterns
- **TestingAndDebugging**: For encryption verification tests
- **PluginPermissionSystem**: For controlling access to encrypted data
