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

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: JsonValue,
    pub duration_ms: u64,
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
