//! DirSoul Input Processing Module
//!
//! Handles multi-modal input processing and conversion to RawMemory.
//! Supports text, voice, image, document, action, and external inputs.
//!
//! # Design Principles
//! - Memory safe: Uses ownership transfer to avoid unnecessary copies
//! - Extensible: Easy to add new input types
//! - Error handling: Comprehensive error types for debugging
//!
//! # Example
//! ```no_run
//! use dirsoul::input::{RawInput, InputProcessor};
//! use dirsoul::models::ContentType;
//!
//! let processor = InputProcessor::new("user123");
//! let input = RawInput::text("Hello, world!");
//! let memory = processor.process_input(input)?;
//! # Ok::<(), dirsoul::DirSoulError>(())
//! ```

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::crypto::EncryptionManager;
use crate::models::{ContentType, NewRawMemory};
use crate::Result;

/// Multi-modal input type for DirSoul
///
/// Represents all possible input types that can be stored in memory.
/// Each variant contains the raw data and optional metadata.
///
/// # Memory Safety
/// - Uses `String` for text data to avoid lifetime issues
/// - Uses `Vec<u8>` for binary data (images, voice, documents)
/// - Large files are handled via file paths, not in-memory copies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RawInput {
    /// Plain text input
    Text {
        content: String,
        metadata: Option<serde_json::Value>,
    },

    /// Voice/audio input
    Voice {
        audio_data: Vec<u8>,        // Raw audio bytes
        format: VoiceFormat,         // e.g., WAV, MP3
        duration_seconds: Option<f32>,
        metadata: Option<serde_json::Value>,
    },

    /// Image input
    Image {
        image_data: Vec<u8>,         // Raw image bytes
        format: ImageFormat,         // e.g., PNG, JPEG
        metadata: Option<serde_json::Value>,
    },

    /// Document input (PDF, Word, etc.)
    Document {
        file_path: PathBuf,
        format: DocumentFormat,
        content: Option<String>,     // Extracted text if available
        metadata: Option<serde_json::Value>,
    },

    /// Action/event input (structured data)
    Action {
        action: String,
        target: String,
        quantity: Option<f32>,
        unit: Option<String>,
        metadata: Option<serde_json::Value>,
    },

    /// External data import
    External {
        source: String,              // Source system
        data: serde_json::Value,     // Structured import data
        metadata: Option<serde_json::Value>,
    },
}

impl RawInput {
    /// Create a new text input
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
            metadata: None,
        }
    }

    /// Create a text input with metadata
    pub fn text_with_metadata(
        content: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self::Text {
            content: content.into(),
            metadata: Some(metadata),
        }
    }

    /// Get the content type for this input
    pub fn content_type(&self) -> ContentType {
        match self {
            Self::Text { .. } => ContentType::Text,
            Self::Voice { .. } => ContentType::Voice,
            Self::Image { .. } => ContentType::Image,
            Self::Document { .. } => ContentType::Document,
            Self::Action { .. } => ContentType::Action,
            Self::External { .. } => ContentType::External,
        }
    }

    /// Get the metadata for this input
    pub fn metadata(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Text { metadata, .. } => metadata.as_ref(),
            Self::Voice { metadata, .. } => metadata.as_ref(),
            Self::Image { metadata, .. } => metadata.as_ref(),
            Self::Document { metadata, .. } => metadata.as_ref(),
            Self::Action { metadata, .. } => metadata.as_ref(),
            Self::External { metadata, .. } => metadata.as_ref(),
        }
    }

    /// Get the size of this input in bytes
    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Text { content, .. } => content.len(),
            Self::Voice { audio_data, .. } => audio_data.len(),
            Self::Image { image_data, .. } => image_data.len(),
            Self::Document { content, .. } => content.as_ref().map(|c| c.len()).unwrap_or(0),
            Self::Action { action, target, unit, .. } => {
                action.len() + target.len() + unit.as_ref().map(|u| u.len()).unwrap_or(0)
            }
            Self::External { source, .. } => source.len(),
        }
    }
}

/// Voice/audio format variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceFormat {
    WAV,
    MP3,
    OGG,
    FLAC,
    OPUS,
    Raw,
}

/// Image format variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    PNG,
    JPEG,
    GIF,
    WebP,
    BMP,
}

/// Document format variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentFormat {
    PDF,
    DOCX,
    TXT,
    MD,
    HTML,
}

/// Input processor for converting RawInput to NewRawMemory
///
/// Handles the conversion logic including optional encryption.
pub struct InputProcessor {
    user_id: String,
    encryption: Option<EncryptionManager>,
}

impl InputProcessor {
    /// Create a new input processor
    ///
    /// # Arguments
    /// * `user_id` - User who owns this input
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            encryption: None,
        }
    }

    /// Set the encryption manager for encrypted storage
    pub fn with_encryption(mut self, encryption: EncryptionManager) -> Self {
        self.encryption = Some(encryption);
        self
    }

    /// Process input and convert to NewRawMemory
    ///
    /// This method takes ownership of the input and converts it to
    /// a database-ready NewRawMemory structure.
    ///
    /// # Arguments
    /// * `input` - Input to process
    ///
    /// # Returns
    /// `NewRawMemory` ready for database insertion
    pub fn process_input(&self, input: RawInput) -> Result<NewRawMemory> {
        info!(
            "Processing input for user '{}': type={:?}, size={} bytes",
            self.user_id,
            input.content_type(),
            input.size_bytes()
        );

        let content_type = input.content_type();
        let user_id = self.user_id.clone();

        match input {
            RawInput::Text { content, metadata } => {
                self.process_text_content(user_id, content_type, content, metadata)
            }

            RawInput::Voice {
                audio_data,
                format,
                duration_seconds,
                metadata,
            } => {
                self.process_voice_content(
                    user_id,
                    audio_data,
                    format,
                    duration_seconds,
                    metadata,
                )
            }

            RawInput::Image {
                image_data,
                format,
                metadata,
            } => self.process_image_content(user_id, image_data, format, metadata),

            RawInput::Document {
                file_path,
                format,
                content,
                metadata,
            } => {
                self.process_document_content(user_id, file_path, format, content, metadata)
            }

            RawInput::Action {
                action,
                target,
                quantity,
                unit,
                metadata,
            } => self.process_action_content(
                user_id, action, target, quantity, unit, metadata,
            ),

            RawInput::External {
                source,
                data,
                metadata,
            } => self.process_external_content(user_id, source, data, metadata),
        }
    }

    /// Process text content
    fn process_text_content(
        &self,
        user_id: String,
        content_type: ContentType,
        content: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<NewRawMemory> {
        debug!("Processing text content: {} bytes", content.len());

        let mut base_metadata = serde_json::json!({
            "source": "text_input",
            "length": content.len(),
        });

        // Merge user metadata
        if let Some(user_metadata) = metadata {
            if let Some(obj) = user_metadata.as_object() {
                if let Some(base_obj) = base_metadata.as_object_mut() {
                    for (key, value) in obj {
                        base_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        // Create memory (encrypted or plaintext based on configuration)
        let memory = if let Some(ref enc) = self.encryption {
            let encrypted = enc.encrypt_string(&content)?;
            NewRawMemory::new_encrypted(user_id, content_type, encrypted.into_bytes())
                .with_metadata(base_metadata)
        } else {
            NewRawMemory::new_plaintext(user_id, content_type, content)
                .with_metadata(base_metadata)
        };

        Ok(memory)
    }

    /// Process voice content
    fn process_voice_content(
        &self,
        user_id: String,
        audio_data: Vec<u8>,
        format: VoiceFormat,
        duration_seconds: Option<f32>,
        metadata: Option<serde_json::Value>,
    ) -> Result<NewRawMemory> {
        debug!("Processing voice content: {} bytes, format={:?}", audio_data.len(), format);

        let mut meta = serde_json::json!({
            "format": format,
            "size_bytes": audio_data.len(),
        });

        if let Some(duration) = duration_seconds {
            meta["duration_seconds"] = serde_json::json!(duration);
        }

        if let Some(user_metadata) = metadata {
            self.merge_metadata(&mut meta, user_metadata);
        }

        // Store base64-encoded audio data
        use base64::Engine;
        let content = base64::engine::general_purpose::STANDARD.encode(&audio_data);

        let memory = if let Some(ref enc) = self.encryption {
            let encrypted = enc.encrypt_string(&content)?;
            NewRawMemory::new_encrypted(user_id, ContentType::Voice, encrypted.into_bytes())
                .with_metadata(meta)
        } else {
            NewRawMemory::new_plaintext(user_id, ContentType::Voice, content)
                .with_metadata(meta)
        };

        Ok(memory)
    }

    /// Process image content
    fn process_image_content(
        &self,
        user_id: String,
        image_data: Vec<u8>,
        format: ImageFormat,
        metadata: Option<serde_json::Value>,
    ) -> Result<NewRawMemory> {
        debug!("Processing image content: {} bytes, format={:?}", image_data.len(), format);

        let mut meta = serde_json::json!({
            "format": format,
            "size_bytes": image_data.len(),
        });

        if let Some(user_metadata) = metadata {
            self.merge_metadata(&mut meta, user_metadata);
        }

        // Store base64-encoded image data
        use base64::Engine;
        let content = base64::engine::general_purpose::STANDARD.encode(&image_data);

        let memory = if let Some(ref enc) = self.encryption {
            let encrypted = enc.encrypt_string(&content)?;
            NewRawMemory::new_encrypted(user_id, ContentType::Image, encrypted.into_bytes())
                .with_metadata(meta)
        } else {
            NewRawMemory::new_plaintext(user_id, ContentType::Image, content)
                .with_metadata(meta)
        };

        Ok(memory)
    }

    /// Process document content
    fn process_document_content(
        &self,
        user_id: String,
        file_path: PathBuf,
        format: DocumentFormat,
        content: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Result<NewRawMemory> {
        debug!(
            "Processing document content: path={:?}, format={:?}",
            file_path, format
        );

        let mut meta = serde_json::json!({
            "format": format,
            "file_path": file_path.to_string_lossy(),
        });

        if let Some(user_metadata) = metadata {
            self.merge_metadata(&mut meta, user_metadata);
        }

        // Store extracted text content if available
        let text_content = content.unwrap_or_else(|| {
            // If no text extracted, store file path reference
            format!("[Document: {}]", file_path.display())
        });

        let memory = if let Some(ref enc) = self.encryption {
            let encrypted = enc.encrypt_string(&text_content)?;
            NewRawMemory::new_encrypted(user_id, ContentType::Document, encrypted.into_bytes())
                .with_metadata(meta)
        } else {
            NewRawMemory::new_plaintext(user_id, ContentType::Document, text_content)
                .with_metadata(meta)
        };

        Ok(memory)
    }

    /// Process action content
    fn process_action_content(
        &self,
        user_id: String,
        action: String,
        target: String,
        quantity: Option<f32>,
        unit: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Result<NewRawMemory> {
        debug!("Processing action: {} {}", action, target);

        let mut meta = serde_json::json!({
            "action": action,
            "target": target,
        });

        if let Some(q) = quantity {
            meta["quantity"] = serde_json::json!(q);
        }

        if let Some(ref u) = unit {
            meta["unit"] = serde_json::json!(u);
        }

        if let Some(user_metadata) = metadata {
            self.merge_metadata(&mut meta, user_metadata);
        }

        // Create a structured text representation
        let content = if let (Some(q), Some(ref u)) = (quantity, unit.as_ref()) {
            format!("{} {} {} {}", action, q, u, target)
        } else if let Some(q) = quantity {
            format!("{} {} {}", action, q, target)
        } else {
            format!("{} {}", action, target)
        };

        let memory = if let Some(ref enc) = self.encryption {
            let encrypted = enc.encrypt_string(&content)?;
            NewRawMemory::new_encrypted(user_id, ContentType::Action, encrypted.into_bytes())
                .with_metadata(meta)
        } else {
            NewRawMemory::new_plaintext(user_id, ContentType::Action, content)
                .with_metadata(meta)
        };

        Ok(memory)
    }

    /// Process external content
    fn process_external_content(
        &self,
        user_id: String,
        source: String,
        data: serde_json::Value,
        metadata: Option<serde_json::Value>,
    ) -> Result<NewRawMemory> {
        debug!("Processing external content from: {}", source);

        let mut meta = serde_json::json!({
            "source": source,
            "imported_at": chrono::Utc::now().to_rfc3339(),
        });

        if let Some(user_metadata) = metadata {
            self.merge_metadata(&mut meta, user_metadata);
        }

        // Convert JSON data to string representation
        let content = serde_json::to_string(&data)
            .map_err(|e| {
                warn!("Failed to serialize external data: {}", e);
                crate::DirSoulError::Serialization(e)
            })?;

        let memory = if let Some(ref enc) = self.encryption {
            let encrypted = enc.encrypt_string(&content)?;
            NewRawMemory::new_encrypted(user_id, ContentType::External, encrypted.into_bytes())
                .with_metadata(meta)
        } else {
            NewRawMemory::new_plaintext(user_id, ContentType::External, content)
                .with_metadata(meta)
        };

        Ok(memory)
    }

    /// Helper function to merge metadata
    fn merge_metadata(&self, base: &mut serde_json::Value, additional: serde_json::Value) {
        if let (Some(base_obj), Some(add_obj)) = (base.as_object_mut(), additional.as_object()) {
            for (key, value) in add_obj {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input() {
        let input = RawInput::text("Hello, world!");
        assert_eq!(input.content_type(), ContentType::Text);
        assert_eq!(input.size_bytes(), 13);
        assert!(input.metadata().is_none());
    }

    #[test]
    fn test_text_input_with_metadata() {
        let meta = serde_json::json!({"source": "test"});
        let input = RawInput::text_with_metadata("Test content", meta);

        assert_eq!(input.content_type(), ContentType::Text);
        assert_eq!(
            input.metadata(),
            Some(&serde_json::json!({"source": "test"}))
        );
    }

    #[test]
    fn test_voice_input() {
        let audio_data = vec![1u8, 2, 3, 4, 5];
        let input = RawInput::Voice {
            audio_data,
            format: VoiceFormat::WAV,
            duration_seconds: Some(1.5),
            metadata: None,
        };

        assert_eq!(input.content_type(), ContentType::Voice);
        assert_eq!(input.size_bytes(), 5);
    }

    #[test]
    fn test_action_input() {
        let input = RawInput::Action {
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: Some(3.0),
            unit: Some("个".to_string()),
            metadata: None,
        };

        assert_eq!(input.content_type(), ContentType::Action);
        assert!(input.size_bytes() > 0);
    }

    #[test]
    fn test_external_input() {
        let data = serde_json::json!({"key": "value"});
        let input = RawInput::External {
            source: "test_system".to_string(),
            data,
            metadata: None,
        };

        assert_eq!(input.content_type(), ContentType::External);
        assert_eq!(input.size_bytes(), 11); // "test_system"
    }

    #[test]
    fn test_input_processor_text() {
        let processor = InputProcessor::new("user123");
        let input = RawInput::text("Test content");

        let memory = processor.process_input(input).unwrap();

        assert_eq!(memory.user_id, "user123");
        assert_eq!(memory.content_type, "text");
        assert_eq!(memory.content, Some("Test content".to_string()));
        assert!(memory.encrypted.is_none());
    }

    #[test]
    fn test_input_processor_action() {
        let processor = InputProcessor::new("user123");
        let input = RawInput::Action {
            action: "eat".to_string(),
            target: "apple".to_string(),
            quantity: Some(3.0),
            unit: Some("个".to_string()),
            metadata: None,
        };

        let memory = processor.process_input(input).unwrap();

        assert_eq!(memory.user_id, "user123");
        assert_eq!(memory.content_type, "action");
        assert_eq!(
            memory.content,
            Some("eat 3 个 apple".to_string())
        );
    }

    #[test]
    fn test_input_processor_external() {
        let processor = InputProcessor::new("user123");
        let data = serde_json::json!({"imported": "data"});
        let input = RawInput::External {
            source: "test".to_string(),
            data,
            metadata: None,
        };

        let memory = processor.process_input(input).unwrap();

        assert_eq!(memory.user_id, "user123");
        assert_eq!(memory.content_type, "external");
        assert!(memory.content.is_some());
    }
}
