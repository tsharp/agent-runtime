use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
#[path = "types_test.rs"]
mod types_test;

/// Unique identifier for workflows
pub type WorkflowId = String;

/// Unique identifier for events
pub type EventId = String;

/// Sequential offset for event ordering
pub type EventOffset = u64;

/// Generic JSON value for flexible data passing
pub type JsonValue = serde_json::Value;

/// Input data passed to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub data: JsonValue,
    pub metadata: AgentInputMetadata,

    /// Optional pre-built chat history. If provided, this is used directly
    /// instead of building messages from data. Allows outer layer to manage
    /// conversation context across multiple agent calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_history: Option<Vec<crate::llm::types::ChatMessage>>,
}

impl AgentInput {
    /// Create a new AgentInput from a text string
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            data: serde_json::json!(text.into()),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
            chat_history: None,
        }
    }

    /// Create a new AgentInput from any JSON-serializable value
    pub fn from_value(value: JsonValue) -> Self {
        Self {
            data: value,
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
            chat_history: None,
        }
    }

    /// Create a new AgentInput with metadata
    pub fn with_metadata(data: JsonValue, metadata: AgentInputMetadata) -> Self {
        Self {
            data,
            metadata,
            chat_history: None,
        }
    }

    /// Create a new AgentInput from existing chat messages
    /// This allows the outer layer to manage conversation history and context.
    ///
    /// # Example
    /// ```
    /// use agent_runtime::{AgentInput, ChatMessage};
    ///
    /// let mut history = vec![
    ///     ChatMessage::system("You are a helpful assistant"),
    ///     ChatMessage::user("What's 2+2?"),
    ///     ChatMessage::assistant("4"),
    ///     ChatMessage::user("What about 3+3?"),
    /// ];
    ///
    /// let input = AgentInput::from_messages(history);
    /// // Agent will continue this conversation
    /// ```
    pub fn from_messages(messages: Vec<crate::llm::types::ChatMessage>) -> Self {
        Self {
            data: serde_json::Value::Null, // Not used when messages provided
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
            chat_history: Some(messages),
        }
    }

    /// Create a new AgentInput from chat messages with custom metadata
    pub fn from_messages_with_metadata(
        messages: Vec<crate::llm::types::ChatMessage>,
        metadata: AgentInputMetadata,
    ) -> Self {
        Self {
            data: serde_json::Value::Null,
            metadata,
            chat_history: Some(messages),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInputMetadata {
    pub step_index: usize,
    pub previous_agent: Option<String>,
}

/// Output data produced by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub data: JsonValue,
    pub metadata: AgentOutputMetadata,

    /// The complete chat history after agent execution.
    /// This includes all messages (system, user, assistant, tool) that were
    /// part of the conversation. Useful for:
    /// - Continuing multi-turn conversations
    /// - Saving and resuming agent state
    /// - Debugging agent behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_history: Option<Vec<crate::llm::types::ChatMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutputMetadata {
    pub agent_name: String,
    pub execution_time_ms: u64,
    pub tool_calls_count: usize,
}

/// Result type for agent execution
pub type AgentResult = Result<AgentOutput, AgentError>;

/// Errors that can occur during agent execution
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum AgentError {
    #[error("Tool execution failed: {0}")]
    ToolError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Execution failed: {0}")]
    ExecutionError(String),
}

/// Tool invocation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub parameters: HashMap<String, JsonValue>,
}

/// Status of a tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// Tool executed successfully and returned data
    #[default]
    Success,

    /// Tool executed successfully but found no data/results
    /// This signals to the LLM: "Don't retry, this is a valid empty result"
    SuccessNoData,

    /// Tool execution failed
    Error,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: JsonValue,
    /// Duration in milliseconds with microsecond precision (e.g., 0.123 ms)
    pub duration_ms: f64,
    /// Status of the execution
    #[serde(default)]
    pub status: ToolStatus,
    /// Optional message explaining the result
    pub message: Option<String>,
}

impl ToolResult {
    /// Create a successful result with data
    pub fn success(output: JsonValue, duration_ms: f64) -> Self {
        Self {
            output,
            duration_ms,
            status: ToolStatus::Success,
            message: None,
        }
    }

    /// Create a successful result with no data
    pub fn success_no_data(message: impl Into<String>, duration_ms: f64) -> Self {
        Self {
            output: JsonValue::Null,
            duration_ms,
            status: ToolStatus::SuccessNoData,
            message: Some(message.into()),
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>, duration_ms: f64) -> Self {
        Self {
            output: JsonValue::Null,
            duration_ms,
            status: ToolStatus::Error,
            message: Some(message.into()),
        }
    }

    /// Add a message to this result
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Result type for tool execution
pub type ToolExecutionResult = Result<ToolResult, ToolError>;

/// Errors that can occur during tool execution
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}
