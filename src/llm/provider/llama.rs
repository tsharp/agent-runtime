use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

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
    pub fn with_http_client(
        base_url: impl Into<String>,
        model: impl Into<String>,
        http_client: HttpClient,
    ) -> Self {
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

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        "llama.cpp"
    }
}

#[async_trait]
impl ChatClient for LlamaClient {
    async fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        // Build llama.cpp-compatible request
        let llama_request = LlamaChatRequest {
            model: self.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            tools: request.tools,
        };

        // Send request
        let response = self
            .http_client
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
            return Err(LlmError::ApiError(format!(
                "Status {}: {}",
                status, error_text
            )));
        }

        // Parse response (same format as OpenAI)
        let llama_response: LlamaChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        // Extract first choice
        let choice = llama_response
            .choices
            .first()
            .ok_or_else(|| LlmError::ParseError("No choices in response".to_string()))?;

        // Convert llama.cpp tool_calls to our ToolCall type
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|tc| super::super::types::ToolCall {
                    id: tc.id.clone(),
                    r#type: tc.r#type.clone(),
                    function: super::super::types::FunctionCall {
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    },
                })
                .collect()
        });

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: llama_response.model.unwrap_or_else(|| self.model.clone()),
            usage: llama_response.usage.map(|u| super::super::types::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
            tool_calls,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
        tx: mpsc::Sender<String>,
    ) -> LlmResult<ChatResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        // Build llama.cpp-compatible request with streaming enabled
        let llama_request = LlamaChatRequest {
            model: self.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            tools: request.tools.clone(),
        };

        // Send request with streaming
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&serde_json::json!({
                "model": llama_request.model,
                "messages": llama_request.messages,
                "temperature": llama_request.temperature,
                "max_tokens": llama_request.max_tokens,
                "top_p": llama_request.top_p,
                "tools": llama_request.tools,
                "stream": true,
            }))
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        // Convert byte stream to text chunks and send through channel
        // Accumulate full response from streaming
        let mut full_content = String::new();
        let mut accumulated_tool_calls: Vec<LlamaToolCall> = Vec::new();
        let mut finish_reason: Option<String> = None;
        let mut model_name: Option<String> = None;
        let mut usage_info: Option<UsageInfo> = None;

        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let bytes = chunk_result.map_err(|e| LlmError::NetworkError(e.to_string()))?;

            // Parse SSE format: "data: {...}\n\n"
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str.trim() == "[DONE]" {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                        // Extract model name if present
                        if model_name.is_none() {
                            model_name = parsed.get("model").and_then(|m| m.as_str()).map(|s| s.to_string());
                        }

                        // Extract usage if present
                        if usage_info.is_none() {
                            if let Some(usage) = parsed.get("usage") {
                                usage_info = serde_json::from_value(usage.clone()).ok();
                            }
                        }

                        if let Some(choice) = parsed.get("choices").and_then(|c| c.get(0)) {
                            // Extract finish_reason
                            if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                                finish_reason = Some(reason.to_string());
                            }

                            // Extract delta
                            if let Some(delta) = choice.get("delta") {
                                // Accumulate content
                                if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                    full_content.push_str(content);
                                    let _ = tx.send(content.to_string()).await;
                                }

                                // Accumulate tool_calls
                                if let Some(tool_calls_array) = delta.get("tool_calls").and_then(|tc| tc.as_array()) {
                                    for tool_call in tool_calls_array {
                                        if let Ok(tc) = serde_json::from_value::<LlamaToolCall>(tool_call.clone()) {
                                            // Check if this tool_call already exists (by index), if so update it
                                            if let Some(index) = tool_call.get("index").and_then(|i| i.as_u64()) {
                                                let idx = index as usize;
                                                if idx < accumulated_tool_calls.len() {
                                                    // Update existing tool call (append arguments)
                                                    if let Some(func_args) = tool_call.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()) {
                                                        accumulated_tool_calls[idx].function.arguments.push_str(func_args);
                                                    }
                                                } else {
                                                    // New tool call
                                                    accumulated_tool_calls.push(tc);
                                                }
                                            } else {
                                                // No index, just add it
                                                accumulated_tool_calls.push(tc);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build response from accumulated streaming data
        let tool_calls = if accumulated_tool_calls.is_empty() {
            None
        } else {
            Some(
                accumulated_tool_calls
                    .iter()
                    .map(|tc| super::super::types::ToolCall {
                        id: tc.id.clone(),
                        r#type: tc.r#type.clone(),
                        function: super::super::types::FunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect(),
            )
        };

        Ok(ChatResponse {
            content: full_content,
            model: model_name.unwrap_or_else(|| self.model.clone()),
            usage: usage_info.map(|u| super::super::types::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason,
            tool_calls,
        })
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

    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
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
    #[serde(default)]
    content: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<LlamaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct LlamaToolCall {
    id: String,
    r#type: String,
    function: LlamaFunction,
}

#[derive(Debug, Deserialize)]
struct LlamaFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}
