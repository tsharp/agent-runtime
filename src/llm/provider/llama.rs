use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use super::super::{ChatClient, ChatRequest, ChatResponse, LlmError, LlmResult};

/// Llama.cpp server client (local or remote)
/// 
/// Compatible with llama.cpp's OpenAI-compatible API server
/// Typically runs on localhost:8080 or similar
pub struct LlamaClient {
    base_url: String,
    model: String,
    http_client: HttpClient,
}

impl LlamaClient {
    /// Create a new llama.cpp client
    /// 
    /// # Arguments
    /// * `base_url` - Base URL of llama.cpp server (e.g., "http://localhost:8080")
    /// * `model` - Model name (optional, llama.cpp usually ignores this)
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            http_client: HttpClient::new(),
        }
    }
    
    /// Create a new llama.cpp client with custom HTTP client
    /// Useful for configuring TLS, timeouts, etc.
    pub fn with_http_client(base_url: impl Into<String>, model: impl Into<String>, http_client: HttpClient) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            http_client,
        }
    }
    
    /// Create a client pointing to localhost:8080 (default llama.cpp port)
    pub fn localhost() -> Self {
        Self::new("http://localhost:8080", "llama")
    }
    
    /// Create a client pointing to localhost with custom port
    pub fn localhost_with_port(port: u16) -> Self {
        Self::new(format!("http://localhost:{}", port), "llama")
    }
    
    /// Create a client with insecure HTTPS (accepts self-signed certificates)
    /// Useful for local development with HTTPS servers
    pub fn insecure(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        let http_client = HttpClient::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to build HTTP client");
        
        Self::with_http_client(base_url, model, http_client)
    }
    
    /// Create localhost client with insecure HTTPS on custom port
    pub fn localhost_insecure(port: u16) -> Self {
        Self::insecure(format!("https://localhost:{}", port), "llama")
    }
}

#[async_trait]
impl ChatClient for LlamaClient {
    async fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        // Build llama.cpp-compatible request
        let llama_request = LlamaChatRequest {
            model: self.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
        };
        
        // Send request
        let response = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&llama_request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;
        
        // Check status
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!("Status {}: {}", status, error_text)));
        }
        
        // Parse response (same format as OpenAI)
        let llama_response: LlamaChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;
        
        // Extract first choice
        let choice = llama_response.choices.first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".to_string()))?;
        
        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: llama_response.model.unwrap_or_else(|| self.model.clone()),
            usage: llama_response.usage.map(|u| super::super::types::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }
    
    fn model(&self) -> &str {
        &self.model
    }
    
    fn provider(&self) -> &str {
        "llama.cpp"
    }
}

// llama.cpp request/response types (OpenAI-compatible)

#[derive(Debug, Serialize)]
struct LlamaChatRequest {
    model: String,
    messages: Vec<super::super::types::ChatMessage>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct LlamaChatResponse {
    model: Option<String>,
    choices: Vec<Choice>,
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
