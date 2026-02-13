use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};


use super::super::{ChatClient, ChatRequest, ChatResponse, LlmError, LlmResult, TextStream};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI chat client
pub struct OpenAIClient {
    api_key: String,
    model: String,
    http_client: HttpClient,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_model(api_key, "gpt-4")
    }
    
    /// Create a new OpenAI client with specific model
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            http_client: HttpClient::new(),
        }
    }
}

#[async_trait]
impl ChatClient for OpenAIClient {
    async fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        // Build OpenAI API request
        let openai_request = OpenAIChatRequest {
            model: self.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
        };
        
        // Send request
        let response = self.http_client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;
        
        // Check status
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => LlmError::AuthenticationFailed(error_text),
                429 => LlmError::RateLimitExceeded,
                _ => LlmError::ApiError(format!("Status {}: {}", status, error_text)),
            });
        }
        
        // Parse response
        let openai_response: OpenAIChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;
        
        // Extract first choice
        let choice = openai_response.choices.first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".to_string()))?;
        
        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: openai_response.model,
            usage: openai_response.usage.map(|u| super::super::types::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }
    
    async fn chat_stream(&self, _request: ChatRequest) -> LlmResult<TextStream> {
        // Simple non-streaming fallback for OpenAI - full implementation would use SSE
        // For now, return error suggesting to use llama.cpp for streaming
        Err(LlmError::ApiError("Streaming not yet implemented for OpenAI - use LlamaClient".to_string()))
    }
    
    fn model(&self) -> &str {
        &self.model
    }
    
    fn provider(&self) -> &str {
        "openai"
    }
}

// OpenAI-specific request/response types

#[derive(Debug, Serialize)]
struct OpenAIChatRequest {
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
struct OpenAIChatResponse {
    model: String,
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
