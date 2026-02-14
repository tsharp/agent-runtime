// Core modules
pub mod agent;
pub mod config;
pub mod error;
pub mod event;
pub mod llm;
pub mod logging;
pub mod retry;
pub mod runtime;
pub mod step;
pub mod step_impls;
pub mod timeout;
pub mod tool;
pub mod tool_loop_detection;
pub mod tools;
pub mod types;
pub mod workflow;

// Re-exports for convenience
pub use agent::{Agent, AgentConfig};
pub use config::{
    LlamaConfig, LlmConfig, LoggingConfig, OpenAIConfig, RetryConfig, RuntimeConfig,
    TimeoutConfigSettings, WorkflowConfig,
};
pub use error::{
    AgentError, AgentErrorCode, ConfigError, ConfigErrorCode, LlmError, LlmErrorCode,
    RuntimeError, ToolError, ToolErrorCode, WorkflowError, WorkflowErrorCode,
};
pub use event::{Event, EventStream, EventType};
pub use llm::{ChatClient, ChatMessage, ChatRequest, ChatResponse, Role};
pub use logging::FileLogger;
pub use retry::RetryPolicy;
pub use runtime::Runtime;
pub use step::{ExecutionContext, Step, StepError, StepInput, StepOutput, StepResult, StepType};
pub use step_impls::{AgentStep, ConditionalStep, SubWorkflowStep, TransformStep};
pub use timeout::{with_timeout, TimeoutConfig};
pub use tool::{NativeTool, Tool, ToolRegistry};
pub use tool_loop_detection::{ToolCallTracker, ToolLoopDetectionConfig};
pub use tools::{McpClient, McpTool, McpToolInfo};
pub use types::*;
pub use workflow::{Workflow, WorkflowBuilder, WorkflowState};

// Prelude module for convenient imports in tests and examples
pub mod prelude {
    pub use crate::agent::{Agent, AgentConfig};
    pub use crate::event::{Event, EventStream, EventType};
    pub use crate::llm::{ChatClient, ChatMessage, ChatRequest, ChatResponse, Role};
    pub use crate::step_impls::{AgentStep, ConditionalStep, SubWorkflowStep, TransformStep};
    pub use crate::tool::{NativeTool, Tool, ToolRegistry};
    pub use crate::types::{AgentInput, AgentOutput, ToolResult, ToolStatus, ToolError as TypesToolError};
    pub use crate::workflow::Workflow;
    
    #[cfg(test)]
    pub use crate::llm::{MockLlmClient, MockResponse, MockToolCall};
    
    #[cfg(not(test))]
    pub use crate::llm::{MockLlmClient, MockResponse, MockToolCall};
}
