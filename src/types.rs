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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// Tool executed successfully and returned data
    Success,
    
    /// Tool executed successfully but found no data/results
    /// This signals to the LLM: "Don't retry, this is a valid empty result"
    SuccessNoData,
    
    /// Tool execution failed
    Error,
}

impl Default for ToolStatus {
    fn default() -> Self {
        Self::Success
    }
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
