#[cfg(test)]
use crate::llm::types::ChatMessage;
use crate::llm::types::{ChatRequest, ChatResponse, FunctionCall, ToolCall, Usage};
use crate::llm::{ChatClient, LlmError};
use async_trait::async_trait;
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
///
/// Supports:
/// - Predefined responses
/// - Tool call simulation
/// - Streaming simulation
/// - Call tracking
/// - Error injection
#[derive(Clone)]
pub struct MockLlmClient {
    responses: Arc<Mutex<Vec<MockResponse>>>,
    calls: Arc<Mutex<Vec<ChatRequest>>>,
    error_on_call: Arc<Mutex<Option<usize>>>, // Fail on nth call
}

/// Mock response configuration
#[derive(Clone, Debug)]
pub struct MockResponse {
    pub content: String,
    pub tool_calls: Vec<MockToolCall>,
    pub finish_reason: String,
}

#[derive(Clone, Debug)]
pub struct MockToolCall {
    pub name: String,
    pub arguments: Value,
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLlmClient {
    /// Create a new empty mock client (call with_response() to add responses)
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(Mutex::new(Vec::new())),
            error_on_call: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new mock client with simple text responses
    pub fn with_responses_vec(responses: Vec<&str>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(
                responses.iter().map(|r| MockResponse::text(r)).collect(),
            )),
            calls: Arc::new(Mutex::new(Vec::new())),
            error_on_call: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a mock client with detailed responses
    pub fn from_mock_responses(responses: Vec<MockResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            calls: Arc::new(Mutex::new(Vec::new())),
            error_on_call: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a mock client that calls a specific tool
    pub fn from_tool_call(tool_name: &str, args: Value) -> Self {
        let response = MockResponse::with_tool_call(tool_name, args);
        Self::from_mock_responses(vec![response])
    }

    /// Create a mock client that calls a tool then responds with text
    pub fn with_tool_then_text(tool_name: &str, args: Value, final_response: &str) -> Self {
        Self::from_mock_responses(vec![
            MockResponse::with_tool_call(tool_name, args),
            MockResponse::text(final_response),
        ])
    }

    /// Add a text response to the chain
    pub fn with_response(self, text: &str) -> Self {
        self.responses
            .lock()
            .unwrap()
            .push(MockResponse::text(text));
        self
    }

    /// Add a tool call response to the chain
    pub fn with_tool_call(self, tool_name: &str, args: Value) -> Self {
        self.responses
            .lock()
            .unwrap()
            .push(MockResponse::with_tool_call(tool_name, args));
        self
    }

    /// Set the client to error on a specific call index
    pub fn error_on_call(self, call_index: usize) -> Self {
        *self.error_on_call.lock().unwrap() = Some(call_index);
        self
    }

    /// Set the client to fail on the nth call (0-indexed)
    pub fn fail_on_call(&self, call_index: usize) {
        *self.error_on_call.lock().unwrap() = Some(call_index);
    }

    /// Get the number of calls made
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// Get a copy of all calls made
    pub fn get_calls(&self) -> Vec<ChatRequest> {
        self.calls.lock().unwrap().clone()
    }

    /// Get the last call made
    pub fn last_call(&self) -> Option<ChatRequest> {
        self.calls.lock().unwrap().last().cloned()
    }

    /// Clear call history
    pub fn clear_calls(&self) {
        self.calls.lock().unwrap().clear();
    }
}

impl MockResponse {
    /// Simple text response
    pub fn text(content: &str) -> Self {
        Self {
            content: content.to_string(),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
        }
    }

    /// Response with a tool call
    pub fn with_tool_call(tool_name: &str, arguments: Value) -> Self {
        Self {
            content: String::new(),
            tool_calls: vec![MockToolCall {
                name: tool_name.to_string(),
                arguments,
            }],
            finish_reason: "tool_calls".to_string(),
        }
    }

    /// Response with multiple tool calls
    pub fn with_tool_calls(tool_calls: Vec<(&str, Value)>) -> Self {
        Self {
            content: String::new(),
            tool_calls: tool_calls
                .into_iter()
                .map(|(name, args)| MockToolCall {
                    name: name.to_string(),
                    arguments: args,
                })
                .collect(),
            finish_reason: "tool_calls".to_string(),
        }
    }
}

#[async_trait]
impl ChatClient for MockLlmClient {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Record the call
        self.calls.lock().unwrap().push(request.clone());

        // Check if we should fail on this call
        let call_index = self.calls.lock().unwrap().len() - 1;
        if let Some(fail_index) = *self.error_on_call.lock().unwrap() {
            if call_index == fail_index {
                return Err(LlmError::NetworkError("Mock network error".to_string()));
            }
        }

        // Get the next response
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Ok(ChatResponse {
                content: "No more mock responses available".to_string(),
                model: "mock-model".to_string(),
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
            });
        }

        let mock_response = responses.remove(0);

        // Convert mock tool calls to actual tool calls
        let tool_calls = if mock_response.tool_calls.is_empty() {
            None
        } else {
            Some(
                mock_response
                    .tool_calls
                    .iter()
                    .enumerate()
                    .map(|(i, tc)| ToolCall {
                        id: format!("call_{}", i),
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name: tc.name.clone(),
                            arguments: serde_json::to_string(&tc.arguments).unwrap(),
                        },
                    })
                    .collect(),
            )
        };

        Ok(ChatResponse {
            content: mock_response.content,
            model: "mock-model".to_string(),
            tool_calls,
            finish_reason: Some(mock_response.finish_reason),
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
        tx: mpsc::Sender<String>,
    ) -> Result<ChatResponse, LlmError> {
        // For streaming, just send the response in chunks
        let response = self.chat(request).await?;

        // Simulate streaming by sending content word by word
        for word in response.content.split_whitespace() {
            let _ = tx.send(format!("{} ", word)).await;
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_simple_response() {
        let client = MockLlmClient::with_responses_vec(vec!["Hello, world!"]);

        let request = ChatRequest::new(vec![ChatMessage::user("Hi")]);

        let response = client.chat(request).await.unwrap();
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(client.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_client_multiple_responses() {
        let client = MockLlmClient::with_responses_vec(vec!["First", "Second", "Third"]);

        let request = ChatRequest::new(vec![ChatMessage::user("Hi")]);

        let r1 = client.chat(request.clone()).await.unwrap();
        assert_eq!(r1.content, "First");

        let r2 = client.chat(request.clone()).await.unwrap();
        assert_eq!(r2.content, "Second");

        let r3 = client.chat(request.clone()).await.unwrap();
        assert_eq!(r3.content, "Third");

        assert_eq!(client.call_count(), 3);
    }

    #[tokio::test]
    async fn test_mock_client_tool_call() {
        let client = MockLlmClient::from_tool_call(
            "calculator",
            json!({"operation": "add", "a": 5, "b": 3}),
        );

        let request = ChatRequest::new(vec![ChatMessage::user("What is 5 + 3?")]);

        let response = client.chat(request).await.unwrap();
        assert!(response.tool_calls.is_some());

        let tool_calls = response.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "calculator");

        let args: Value = serde_json::from_str(&tool_calls[0].function.arguments).unwrap();
        assert_eq!(args["operation"], "add");
        assert_eq!(args["a"], 5);
        assert_eq!(args["b"], 3);
    }

    #[tokio::test]
    async fn test_mock_client_error_injection() {
        let client = MockLlmClient::with_responses_vec(vec!["First", "Second", "Third"]);
        client.fail_on_call(1); // Fail on second call

        let request = ChatRequest::new(vec![ChatMessage::user("Hi")]);

        // First call succeeds
        let r1 = client.chat(request.clone()).await;
        assert!(r1.is_ok());

        // Second call fails
        let r2 = client.chat(request.clone()).await;
        assert!(r2.is_err());

        // Third call succeeds
        let r3 = client.chat(request.clone()).await;
        assert!(r3.is_ok());
    }

    #[tokio::test]
    async fn test_mock_client_call_tracking() {
        let client = MockLlmClient::with_responses_vec(vec!["Response 1", "Response 2"]);

        let req1 = ChatRequest::new(vec![ChatMessage::user("Question 1")]);
        let req2 = ChatRequest::new(vec![ChatMessage::user("Question 2")]);

        client.chat(req1).await.unwrap();
        client.chat(req2).await.unwrap();

        let calls = client.get_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].messages[0].content, "Question 1");
        assert_eq!(calls[1].messages[0].content, "Question 2");
    }
}
