use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

pub mod provider;
pub mod types;

pub use provider::{OpenAIClient, LlamaClient};
pub use types::{ChatMessage, ChatRequest, ChatResponse, Role};

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

/// Stream of text chunks from LLM
pub type TextStream = Pin<Box<dyn Stream<Item = LlmResult<String>> + Send>>;

/// Errors that can occur during LLM operations
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Response parsing error: {0}")]
    ParseError(String),
}

/// Generic trait for LLM chat clients
#[async_trait]
pub trait ChatClient: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse>;
    
    /// Stream a chat completion request (yields text chunks as they arrive)
    async fn chat_stream(&self, request: ChatRequest) -> LlmResult<TextStream>;
    
    /// Get the model name this client uses
    fn model(&self) -> &str;
    
    /// Get the provider name (e.g., "openai", "llama.cpp")
    fn provider(&self) -> &str;
}
