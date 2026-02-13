use async_trait::async_trait;

pub mod provider;
pub mod types;

pub use provider::{OpenAIClient, LlamaClient};
pub use types::{ChatMessage, ChatRequest, ChatResponse, Role};

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

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
    
    /// Get the model name this client uses
    fn model(&self) -> &str;
    
    /// Get the provider name (e.g., "openai", "llama.cpp")
    fn provider(&self) -> &str;
}
