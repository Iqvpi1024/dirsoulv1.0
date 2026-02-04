//! Prompt Manager Module - Externalized Prompt Templates
//!
//! This module implements prompt template management, allowing prompts to be
//! stored externally in files rather than hardcoded in the application code.
//!
//! # Design Principles (from chat88.md)
//! - **Prompt Externalization**: Avoid hardcoding prompts in source code
//! - **User Customization**: Users can modify prompts without recompiling
//! - **Template Variables**: Support for variable substitution ({{variable}})
//!
//! # Example
//! ```no_run
//! use dirsoul::PromptManager;
//! use std::collections::HashMap;
//!
//! let mut manager = PromptManager::with_dir("/path/to/prompts")?;
//! let mut vars = HashMap::new();
//! vars.insert("context", "User input text");
//! let rendered = manager.render_prompt("event_extraction", vars)?;
//! # Ok::<(), dirsoul::DirSoulError>(())
//! ```

use crate::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Default prompts directory
const DEFAULT_PROMPTS_DIR: &str = "prompts";

/// Prompt Manager - loads and renders prompt templates from files
///
/// Templates use double-brace syntax for variables: `{{variable_name}}`
pub struct PromptManager {
    prompts_dir: PathBuf,
    /// Cache for loaded prompts to avoid repeated disk I/O
    cache: HashMap<String, String>,
}

impl PromptManager {
    /// Create a new PromptManager with the default prompts directory
    pub fn new() -> Result<Self> {
        Self::with_dir(DEFAULT_PROMPTS_DIR)
    }

    /// Create a new PromptManager with a custom prompts directory
    pub fn with_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let prompts_dir = dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if !prompts_dir.exists() {
            fs::create_dir_all(&prompts_dir).map_err(|e| {
                crate::error::DirSoulError::Io(e)
            })?;
        }

        Ok(Self {
            prompts_dir,
            cache: HashMap::new(),
        })
    }

    /// Load a prompt template by name
    ///
    /// This loads the template from `{prompts_dir}/{name}.txt`
    /// Caches the result for subsequent calls.
    pub fn load_prompt(&mut self, name: &str) -> Result<String> {
        // Check cache first
        if let Some(cached) = self.cache.get(name) {
            return Ok(cached.clone());
        }

        // Build file path
        let file_path = self.prompts_dir.join(format!("{}.txt", name));

        // Read file
        let content = fs::read_to_string(&file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                crate::error::DirSoulError::NotFound(format!(
                    "Prompt template not found: {}",
                    file_path.display()
                ))
            } else {
                crate::error::DirSoulError::Io(e)
            }
        })?;

        // Cache the result
        self.cache.insert(name.to_string(), content.clone());

        Ok(content)
    }

    /// Load a prompt template without caching
    ///
    /// This bypasses the cache and always reads from disk.
    pub fn load_prompt_fresh(&self, name: &str) -> Result<String> {
        let file_path = self.prompts_dir.join(format!("{}.txt", name));

        fs::read_to_string(&file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                crate::error::DirSoulError::NotFound(format!(
                    "Prompt template not found: {}",
                    file_path.display()
                ))
            } else {
                crate::error::DirSoulError::Io(e)
            }
        })
    }

    /// Render a prompt template with variable substitution
    ///
    /// Replaces `{{variable}}` placeholders with values from the vars map.
    pub fn render_prompt(
        &mut self,
        name: &str,
        vars: HashMap<&str, &str>,
    ) -> Result<String> {
        let template = self.load_prompt(name)?;

        let mut result = template;

        // Replace all variables
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    /// Render a prompt template with string-based variables
    ///
    /// Convenience method for String-based HashMap
    pub fn render_prompt_string(
        &mut self,
        name: &str,
        vars: &HashMap<String, String>,
    ) -> Result<String> {
        let template = self.load_prompt(name)?;

        let mut result = template;

        // Replace all variables
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    /// Clear the prompt cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the prompts directory path
    pub fn prompts_dir(&self) -> &Path {
        &self.prompts_dir
    }

    /// List all available prompt templates
    pub fn list_prompts(&self) -> Result<Vec<String>> {
        let entries = fs::read_dir(&self.prompts_dir).map_err(|e| {
            crate::error::DirSoulError::Io(e)
        })?;

        let mut prompts = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| crate::error::DirSoulError::Io(e))?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "txt").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        prompts.push(name.to_string());
                    }
                }
            }
        }

        prompts.sort();
        Ok(prompts)
    }

    /// Check if a prompt template exists
    pub fn has_prompt(&self, name: &str) -> bool {
        let file_path = self.prompts_dir.join(format!("{}.txt", name));
        file_path.exists()
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::with_dir(DEFAULT_PROMPTS_DIR)
            .expect("Failed to create default PromptManager")
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a temporary directory with test prompts
    fn setup_test_prompts() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path().join("prompts");
        fs::create_dir_all(&prompts_dir).unwrap();

        // Create test prompt templates
        fs::write(
            prompts_dir.join("test_simple.txt"),
            "This is a simple prompt template.",
        )
        .unwrap();

        fs::write(
            prompts_dir.join("test_variables.txt"),
            "Hello {{name}}, your role is {{role}}. Context: {{context}}",
        )
        .unwrap();

        fs::write(
            prompts_dir.join("test_multiline.txt"),
            "System: You are a helpful assistant.\n\nUser: {{user_input}}\n\nPlease respond.",
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_prompt_manager_creation() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let manager = PromptManager::with_dir(&prompts_dir).unwrap();
        assert_eq!(manager.prompts_dir(), &prompts_dir);
    }

    #[test]
    fn test_load_simple_prompt() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();
        let prompt = manager.load_prompt("test_simple").unwrap();

        assert_eq!(prompt, "This is a simple prompt template.");
    }

    #[test]
    fn test_load_nonexistent_prompt() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();
        let result = manager.load_prompt("nonexistent");

        assert!(result.is_err());
        match result {
            Err(crate::error::DirSoulError::NotFound(msg)) => {
                assert!(msg.contains("nonexistent"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_render_prompt_with_variables() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        let mut vars = HashMap::new();
        vars.insert("name", "Alice");
        vars.insert("role", "developer");
        vars.insert("context", "building a memory system");

        let rendered = manager.render_prompt("test_variables", vars).unwrap();

        assert_eq!(
            rendered,
            "Hello Alice, your role is developer. Context: building a memory system"
        );
    }

    #[test]
    fn test_render_prompt_string_vars() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Bob".to_string());
        vars.insert("role".to_string(), "designer".to_string());
        vars.insert("context".to_string(), "UI design".to_string());

        let rendered = manager.render_prompt_string("test_variables", &vars).unwrap();

        assert_eq!(
            rendered,
            "Hello Bob, your role is designer. Context: UI design"
        );
    }

    #[test]
    fn test_render_prompt_partial_variables() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        let mut vars = HashMap::new();
        vars.insert("name", "Charlie");
        // 'role' and 'context' not provided

        let rendered = manager.render_prompt("test_variables", vars).unwrap();

        // Unfilled variables remain as placeholders
        assert_eq!(
            rendered,
            "Hello Charlie, your role is {{role}}. Context: {{context}}"
        );
    }

    #[test]
    fn test_prompt_caching() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        // First load
        let prompt1 = manager.load_prompt("test_simple").unwrap();

        // Second load should be cached
        let prompt2 = manager.load_prompt("test_simple").unwrap();

        assert_eq!(prompt1, prompt2);
        assert_eq!(manager.cache.len(), 1);
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        manager.load_prompt("test_simple").unwrap();
        assert_eq!(manager.cache.len(), 1);

        manager.clear_cache();
        assert_eq!(manager.cache.len(), 0);
    }

    #[test]
    fn test_load_prompt_fresh() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        // Load into cache
        manager.load_prompt("test_simple").unwrap();

        // Load fresh bypasses cache
        let fresh = manager.load_prompt_fresh("test_simple").unwrap();

        assert_eq!(fresh, "This is a simple prompt template.");
        // Cache should still have 1 entry
        assert_eq!(manager.cache.len(), 1);
    }

    #[test]
    fn test_list_prompts() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let manager = PromptManager::with_dir(&prompts_dir).unwrap();

        let prompts = manager.list_prompts().unwrap();

        assert_eq!(prompts.len(), 3);
        assert!(prompts.contains(&"test_simple".to_string()));
        assert!(prompts.contains(&"test_variables".to_string()));
        assert!(prompts.contains(&"test_multiline".to_string()));
    }

    #[test]
    fn test_has_prompt() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let manager = PromptManager::with_dir(&prompts_dir).unwrap();

        assert!(manager.has_prompt("test_simple"));
        assert!(manager.has_prompt("test_variables"));
        assert!(!manager.has_prompt("nonexistent"));
    }

    #[test]
    fn test_multiline_prompt() {
        let temp_dir = setup_test_prompts();
        let prompts_dir = temp_dir.path().join("prompts");

        let mut manager = PromptManager::with_dir(&prompts_dir).unwrap();

        let mut vars = HashMap::new();
        vars.insert("user_input", "Tell me about memory");

        let rendered = manager.render_prompt("test_multiline", vars).unwrap();

        assert!(rendered.contains("System: You are a helpful assistant."));
        assert!(rendered.contains("User: Tell me about memory"));
        assert!(rendered.contains("Please respond."));
    }

    #[test]
    fn test_default_prompt_manager() {
        let manager = PromptManager::default();
        assert_eq!(manager.prompts_dir(), Path::new(DEFAULT_PROMPTS_DIR));
    }
}
