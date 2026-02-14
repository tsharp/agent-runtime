use std::fmt;

/// Main error type for the agent runtime system
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Error during workflow execution
    Workflow(WorkflowError),
    
    /// Error during agent execution
    Agent(AgentError),
    
    /// Error from LLM provider
    Llm(LlmError),
    
    /// Error during tool execution
    Tool(ToolError),
    
    /// Configuration validation error
    Config(ConfigError),
    
    /// Retry attempts exhausted
    RetryExhausted {
        operation: String,
        attempts: u32,
        last_error: Box<RuntimeError>,
    },
    
    /// Operation timed out
    Timeout {
        operation: String,
        duration_ms: u64,
    },
}

/// Workflow-specific errors
#[derive(Debug, Clone)]
pub struct WorkflowError {
    pub code: WorkflowErrorCode,
    pub message: String,
    pub step_id: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowErrorCode {
    StepExecutionFailed,
    InvalidStepOutput,
    CycleDetected,
    MaxIterationsExceeded,
    ConditionalEvaluationFailed,
}

/// Agent-specific errors
#[derive(Debug, Clone)]
pub struct AgentError {
    pub code: AgentErrorCode,
    pub message: String,
    pub agent_name: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentErrorCode {
    ExecutionFailed,
    InvalidInput,
    InvalidOutput,
    ToolExecutionFailed,
    MaxToolIterationsExceeded,
    MissingLlmClient,
    MissingSystemPrompt,
}

/// LLM provider errors
#[derive(Debug, Clone)]
pub struct LlmError {
    pub code: LlmErrorCode,
    pub message: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmErrorCode {
    NetworkError,
    AuthenticationFailed,
    RateLimitExceeded,
    InvalidRequest,
    InvalidResponse,
    ModelNotFound,
    ContextLengthExceeded,
    ServerError,
    ParseError,
}

/// Tool execution errors
#[derive(Debug, Clone)]
pub struct ToolError {
    pub code: ToolErrorCode,
    pub message: String,
    pub tool_name: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolErrorCode {
    InvalidParameters,
    ExecutionFailed,
    Timeout,
    NotFound,
    McpConnectionFailed,
    McpToolCallFailed,
}

/// Configuration validation errors
#[derive(Debug, Clone)]
pub struct ConfigError {
    pub code: ConfigErrorCode,
    pub message: String,
    pub field: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorCode {
    MissingRequiredField,
    InvalidValue,
    ValidationFailed,
    FileNotFound,
    ParseError,
}

// Implement Display for all error types
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Workflow(e) => write!(f, "Workflow error: {}", e),
            RuntimeError::Agent(e) => write!(f, "Agent error: {}", e),
            RuntimeError::Llm(e) => write!(f, "LLM error: {}", e),
            RuntimeError::Tool(e) => write!(f, "Tool error: {}", e),
            RuntimeError::Config(e) => write!(f, "Configuration error: {}", e),
            RuntimeError::RetryExhausted { operation, attempts, last_error } => {
                write!(f, "Retry exhausted for '{}' after {} attempts: {}", operation, attempts, last_error)
            }
            RuntimeError::Timeout { operation, duration_ms } => {
                write!(f, "Operation '{}' timed out after {}ms", operation, duration_ms)
            }
        }
    }
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(step_id) = &self.step_id {
            write!(f, " (step: {})", step_id)?;
        }
        if let Some(context) = &self.context {
            write!(f, " - {}", context)?;
        }
        Ok(())
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(agent_name) = &self.agent_name {
            write!(f, " (agent: {})", agent_name)?;
        }
        if let Some(context) = &self.context {
            write!(f, " - {}", context)?;
        }
        Ok(())
    }
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(provider) = &self.provider {
            write!(f, " (provider: {})", provider)?;
        }
        if let Some(model) = &self.model {
            write!(f, " [model: {}]", model)?;
        }
        if self.retryable {
            write!(f, " (retryable)")?;
        }
        Ok(())
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(tool_name) = &self.tool_name {
            write!(f, " (tool: {})", tool_name)?;
        }
        if let Some(context) = &self.context {
            write!(f, " - {}", context)?;
        }
        Ok(())
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(field) = &self.field {
            write!(f, " (field: {})", field)?;
        }
        Ok(())
    }
}

// Implement std::error::Error
impl std::error::Error for RuntimeError {}
impl std::error::Error for WorkflowError {}
impl std::error::Error for AgentError {}
impl std::error::Error for LlmError {}
impl std::error::Error for ToolError {}
impl std::error::Error for ConfigError {}

// Helper methods for LlmError
impl LlmError {
    /// Check if this error is retryable (network issues, rate limits, server errors)
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.code,
            LlmErrorCode::NetworkError
                | LlmErrorCode::RateLimitExceeded
                | LlmErrorCode::ServerError
        )
    }
    
    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self {
            code: LlmErrorCode::NetworkError,
            message: message.into(),
            provider: None,
            model: None,
            retryable: true,
        }
    }
    
    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self {
            code: LlmErrorCode::RateLimitExceeded,
            message: message.into(),
            provider: None,
            model: None,
            retryable: true,
        }
    }
    
    /// Create a server error
    pub fn server_error(message: impl Into<String>) -> Self {
        Self {
            code: LlmErrorCode::ServerError,
            message: message.into(),
            provider: None,
            model: None,
            retryable: true,
        }
    }
}

// Conversion helpers
impl From<WorkflowError> for RuntimeError {
    fn from(e: WorkflowError) -> Self {
        RuntimeError::Workflow(e)
    }
}

impl From<AgentError> for RuntimeError {
    fn from(e: AgentError) -> Self {
        RuntimeError::Agent(e)
    }
}

impl From<LlmError> for RuntimeError {
    fn from(e: LlmError) -> Self {
        RuntimeError::Llm(e)
    }
}

impl From<ToolError> for RuntimeError {
    fn from(e: ToolError) -> Self {
        RuntimeError::Tool(e)
    }
}

impl From<ConfigError> for RuntimeError {
    fn from(e: ConfigError) -> Self {
        RuntimeError::Config(e)
    }
}
