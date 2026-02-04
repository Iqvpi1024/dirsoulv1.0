//! LLM Provider Module - Model Adapter Pattern
//!
//! Abstracts LLM backend (Ollama, OpenAI-compatible APIs) to support
//! user-selectable models without code changes.
//!
//! # Design Principles (from chat88.md)
//! - **Dual Model Strategy**: Separate Embedding (fixed) from Inference (user-selectable)
//! - **Configuration-driven**: User switches models via config file, no code changes
//! - **Extensible**: New providers can be added by implementing LLMProvider trait
//!
//! # Architecture
//! ```text
//! LLMProvider (trait)
//!     ├── OllamaProvider (local models: phi4-mini, deepseek-r1, llama-3, etc.)
//!     ├── OpenAICompatibleProvider (APIs: DeepSeek V3, SiliconFlow, OpenAI, etc.)
//!     └── Future: AzureOpenAIProvider, AnthropicProvider, etc.
//! ```

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::Result;

/// Default Ollama host
const DEFAULT_OLLAMA_HOST: &str = "http://127.0.0.1:11434";

/// LLM chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// LLM chat response
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ChatResponse {
    /// Ollama format
    Ollama(OllamaChatResponse),
    /// OpenAI-compatible format
    OpenAI(OpenAIChatResponse),
}

/// Ollama chat response format
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaChatResponse {
    pub response: String,
    pub done: bool,
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,
    #[serde(default)]
    pub eval_count: Option<u32>,
}

/// OpenAI-compatible chat response format
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: usize,
    pub message: ChatMessageContent,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessageContent {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone,Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Embedding response
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingResponse {
    /// Ollama format
    Ollama(OllamaEmbeddingResponse),
    /// OpenAI-compatible format
    OpenAI(OpenAIEmbeddingResponse),
}

/// Ollama embedding response
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaEmbeddingResponse {
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingData {
    pub embedding: Vec<f32>,
    pub index: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI-compatible embedding response
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIEmbeddingResponse {
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Option<EmbeddingUsage>,
}

// ============================================================================
// Streaming Types
// ============================================================================

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

// ============================================================================
// LLM Provider Trait
// ============================================================================

/// LLM Provider trait - abstracts different LLM backends
///
/// This trait enables swapping LLM providers (Ollama, OpenAI-compatible APIs, etc.)
/// without changing application code. Users can switch models via config file.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse>;

    /// Send a streaming chat completion request
    async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamChunk>>;

    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get the model name being used
    fn model_name(&self) -> String;

    /// Health check for the provider
    async fn health_check(&self) -> Result<bool>;
}

/// Extract response text from ChatResponse (helper function)
pub fn extract_response_text(response: &ChatResponse) -> String {
    match response {
        ChatResponse::Ollama(ollama) => ollama.response.clone(),
        ChatResponse::OpenAI(openai) => {
            openai.choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default()
        }
    }
}

// ============================================================================
// Model Configuration
// ============================================================================

/// Model provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Provider type: "ollama" or "openai_compatible"
    pub provider: String,
    
    /// Model name (e.g., "phi4-mini", "nomic-embed-text:v1.5", "deepseek-chat")
    pub model: String,
    
    /// Ollama-specific configuration
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,
    
    /// OpenAI-compatible API configuration
    #[serde(default)]
    pub openai_compatible: Option<OpenAICompatibleConfig>,
}

/// Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama host URL (default: http://127.0.0.1:11434)
    #[serde(default = "default_ollama_host")]
    pub host: String,
}

fn default_ollama_host() -> String {
    DEFAULT_OLLAMA_HOST.to_string()
}

/// OpenAI-compatible API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatibleConfig {
    /// Base URL for the API (e.g., https://api.deepseek.com)
    pub base_url: String,
    
    /// API key for authentication
    pub api_key: String,
}

// ============================================================================
// Ollama Provider Implementation
// ============================================================================

/// Ollama provider for local models
///
/// Supports running local models like phi4-mini, deepseek-r1, llama-3, etc.
/// through Ollama API (http://localhost:11434 by default).
pub struct OllamaProvider {
    client: Client,
    host: String,
    model: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(host: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            host: host.into(),
            model: model.into(),
        }
    }

    /// Build the full API URL for an endpoint
    fn url(&self, endpoint: &str) -> String {
        format!("{}/api/{}", self.host.trim_end_matches('/'), endpoint)
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse> {
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            options: Option<ChatOptions>,
        }

        #[derive(Serialize)]
        struct ChatOptions {
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            num_predict: Option<u32>,
        }

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature,
                num_predict: max_tokens,
            }),
        };

        let response = self
            .client
            .post(&self.url("chat"))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "Ollama chat failed: {}",
                response.status()
            )));
        }

        let ollama_response: OllamaChatResponse = response.json().await?;
        Ok(ChatResponse::Ollama(ollama_response))
    }

    async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        use tokio::sync::mpsc;

        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            options: Option<ChatOptions>,
        }

        #[derive(Serialize)]
        struct ChatOptions {
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            num_predict: Option<u32>,
        }

        let (tx, rx) = mpsc::channel(100);

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            options: Some(ChatOptions {
                temperature,
                num_predict: max_tokens,
            }),
        };

        let url = self.url("chat");
        let client = self.client.clone();
        let request_json = serde_json::to_value(&request)
            .map_err(|e| crate::error::DirSoulError::Serialization(e))?;

        tokio::spawn(async move {
            if let Err(e) = Self::stream_chat_impl(client, url, request_json, tx).await {
                tracing::error!("Ollama stream error: {:?}", e);
            }
        });

        Ok(rx)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            prompt: String,
        }

        let request = EmbedRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self
            .client
            .post(&self.url("embed"))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "Ollama embed failed: {}",
                response.status()
            )));
        }

        let ollama_response: OllamaEmbeddingResponse = response.json().await?;
        Ok(ollama_response.embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Ollama doesn't support batch embeddings, so we process sequentially
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    fn model_name(&self) -> String {
        self.model.clone()
    }

    async fn health_check(&self) -> Result<bool> {
        let response = self.client.get(&self.url("tags")).send().await?;
        Ok(response.status().is_success())
    }
}

impl OllamaProvider {
    async fn stream_chat_impl(
        client: Client,
        url: String,
        request: serde_json::Value,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        use futures_util::StreamExt;

        let resp = client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "Stream request failed: {}",
                resp.status()
            )));
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let data = String::from_utf8_lossy(&chunk);
            
            for line in data.lines() {
                if line.is_empty() {
                    continue;
                }
                buffer.push_str(line);
                
                // Try to parse as JSON
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&buffer) {
                    buffer.clear();
                    
                    let done = value.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
                    let content = value.get("response")
                        .and_then(|r| r.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    if tx.send(StreamChunk { content, done }).await.is_err() {
                        return Ok(()); // Receiver dropped
                    }
                    
                    if done {
                        return Ok(());
                    }
                }
            }
        }
        
        Ok(())
    }
}

// ============================================================================
// OpenAI-Compatible Provider Implementation
// ============================================================================

/// OpenAI-compatible API provider
///
/// Supports APIs that follow the OpenAI format:
/// - DeepSeek V3 (https://api.deepseek.com)
/// - SiliconFlow (https://api.siliconflow.cn)
/// - OpenAI (https://api.openai.com)
/// - Any other OpenAI-compatible API
pub struct OpenAICompatibleProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAICompatibleProvider {
    /// Create a new OpenAI-compatible provider
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Build the full API URL for an endpoint
    fn url(&self, endpoint: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/v1/{}", base, endpoint.trim_start_matches('/'))
    }

    /// Get authorization header value
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

#[async_trait]
impl LLMProvider for OpenAICompatibleProvider {
    async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse> {
        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
        }

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature,
            max_tokens,
        };

        let response = self
            .client
            .post(&self.url("chat/completions"))
            .header("Authorization", self.auth_header())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "OpenAI-compatible chat failed: {} - {}",
                status,
                error_text
            )));
        }

        let openai_response: OpenAIChatResponse = response.json().await?;
        Ok(ChatResponse::OpenAI(openai_response))
    }

    async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
        use tokio::sync::mpsc;

        #[derive(Serialize)]
        struct ChatRequest {
            model: String,
            messages: Vec<ChatMessage>,
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
        }

        let (tx, rx) = mpsc::channel(100);

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            temperature,
            max_tokens,
        };

        let url = self.url("chat/completions");
        let client = self.client.clone();
        let auth = self.auth_header();
        let request_json = serde_json::to_value(&request)
            .map_err(|e| crate::error::DirSoulError::Serialization(e))?;

        tokio::spawn(async move {
            if let Err(e) = Self::stream_chat_impl(client, url, auth, request_json, tx).await {
                tracing::error!("OpenAI-compatible stream error: {:?}", e);
            }
        });

        Ok(rx)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            input: String,
        }

        let request = EmbedRequest {
            model: self.model.clone(),
            input: text.to_string(),
        };

        let response = self
            .client
            .post(&self.url("embeddings"))
            .header("Authorization", self.auth_header())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "OpenAI-compatible embed failed: {} - {}",
                status,
                error_text
            )));
        }

        let openai_response: OpenAIEmbeddingResponse = response.json().await?;
        
        openai_response
            .data
            .into_iter()
            .find(|d| d.index == 0)
            .map(|d| Ok(d.embedding))
            .unwrap_or_else(|| Err(crate::error::DirSoulError::ExternalError(format!("No embedding in response"))))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            input: Vec<String>,
        }

        let request = EmbedRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let response = self
            .client
            .post(&self.url("embeddings"))
            .header("Authorization", self.auth_header())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "OpenAI-compatible embed batch failed: {} - {}",
                status,
                error_text
            )));
        }

        let openai_response: OpenAIEmbeddingResponse = response.json().await?;
        
        let mut embeddings = vec![Vec::new(); texts.len()];
        for data in openai_response.data {
            if data.index < embeddings.len() {
                embeddings[data.index] = data.embedding;
            }
        }
        
        Ok(embeddings)
    }

    fn model_name(&self) -> String {
        self.model.clone()
    }

    async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(&self.url("models"))
            .header("Authorization", self.auth_header())
            .send()
            .await?;
        Ok(response.status().is_success())
    }
}

impl OpenAICompatibleProvider {
    async fn stream_chat_impl(
        client: Client,
        url: String,
        auth: String,
        request: serde_json::Value,
        tx: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> Result<()> {
        use futures_util::StreamExt;

        let resp = client
            .post(&url)
            .header("Authorization", auth)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(crate::error::DirSoulError::ExternalError(format!(
                "Stream request failed: {}",
                resp.status()
            )));
        }

        let mut stream = resp.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let data = String::from_utf8_lossy(&chunk);
            
            for line in data.lines() {
                let line = line.strip_prefix("data: ").unwrap_or(line);
                
                if line == "[DONE]" {
                    let _ = tx.send(StreamChunk {
                        content: String::new(),
                        done: true,
                    }).await;
                    return Ok(());
                }
                
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                    let content = value
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    
                    let done = value
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("finish_reason"))
                        .is_some();
                    
                    if tx.send(StreamChunk { content, done }).await.is_err() {
                        return Ok(()); // Receiver dropped
                    }
                    
                    if done {
                        return Ok(());
                    }
                }
            }
        }
        
        Ok(())
    }
}

// ============================================================================
// Model Provider Factory
// ============================================================================

/// Factory for creating LLM providers from configuration
///
/// # Example
/// See `OllamaProvider` and `OpenAICompatibleProvider` for direct usage examples.
pub struct ModelProviderFactory;

impl ModelProviderFactory {
    /// Create an LLM provider from configuration
    pub fn create_provider(config: ModelConfig) -> Result<Arc<dyn LLMProvider>> {
        match config.provider.as_str() {
            "ollama" => {
                let ollama_config = config.ollama.unwrap_or_default();
                let provider = OllamaProvider::new(ollama_config.host, config.model);
                Ok(Arc::new(provider))
            }
            "openai_compatible" => {
                let api_config = config
                    .openai_compatible
                    .ok_or_else(|| crate::error::DirSoulError::Config(format!("Missing openai_compatible configuration")))?;
                let provider = OpenAICompatibleProvider::new(
                    api_config.base_url,
                    api_config.api_key,
                    config.model,
                );
                Ok(Arc::new(provider))
            }
            _ => Err(crate::error::DirSoulError::Config(format!("Unknown provider: {}", config.provider))),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: default_ollama_host(),
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock provider for testing
    struct MockProvider {
        model: String,
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat(
            &self,
            _messages: Vec<ChatMessage>,
            _temperature: Option<f32>,
            _max_tokens: Option<u32>,
        ) -> Result<ChatResponse> {
            Ok(ChatResponse::Ollama(OllamaChatResponse {
                response: "Mock response".to_string(),
                done: true,
                prompt_eval_count: None,
                eval_count: None,
            }))
        }

        async fn stream_chat(
            &self,
            _messages: Vec<ChatMessage>,
            _temperature: Option<f32>,
            _max_tokens: Option<u32>,
        ) -> Result<tokio::sync::mpsc::Receiver<StreamChunk>> {
            let (tx, rx) = tokio::sync::mpsc::channel(10);
            let _ = tx.send(StreamChunk {
                content: "Mock stream".to_string(),
                done: true,
            })
            .await
            .map_err(|e| crate::error::DirSoulError::ExternalError(format!("Send error: {}", e)))?;
            Ok(rx)
        }

        async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![0.0; 512])
        }

        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.0; 512]).collect())
        }

        fn model_name(&self) -> String {
            self.model.clone()
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn test_chat_message_constructors() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let system_msg = ChatMessage::system("You are helpful");
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content, "You are helpful");

        let assistant_msg = ChatMessage::assistant("Hi there");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Hi there");
    }

    #[test]
    fn test_extract_response_text() {
        let ollama_response = ChatResponse::Ollama(OllamaChatResponse {
            response: "Ollama text".to_string(),
            done: true,
            prompt_eval_count: None,
            eval_count: None,
        });
        assert_eq!(extract_response_text(&ollama_response), "Ollama text");

        let openai_response = ChatResponse::OpenAI(OpenAIChatResponse {
            id: None,
            object: None,
            created: None,
            model: None,
            choices: vec![Choice {
                index: 0,
                message: ChatMessageContent {
                    role: "assistant".to_string(),
                    content: "OpenAI text".to_string(),
                },
                finish_reason: None,
            }],
            usage: None,
        });
        assert_eq!(extract_response_text(&openai_response), "OpenAI text");
    }

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider {
            model: "mock-model".to_string(),
        };

        // Test chat
        let messages = vec![ChatMessage::user("Test")];
        let response = provider.chat(messages, None, None).await.unwrap();
        let text = extract_response_text(&response);
        assert_eq!(text, "Mock response");

        // Test embed
        let embedding = provider.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), 512);

        // Test embed_batch
        let texts = vec!["a".to_string(), "b".to_string()];
        let embeddings = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 512);

        // Test model_name
        assert_eq!(provider.model_name(), "mock-model");

        // Test health_check
        assert!(provider.health_check().await.unwrap());
    }

    #[test]
    fn test_model_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.host, "http://127.0.0.1:11434");
    }

    #[test]
    fn test_stream_chunk() {
        let chunk = StreamChunk {
            content: "Hello".to_string(),
            done: false,
        };
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.done);
    }
}
