// Core modules
pub mod agent;
pub mod config;
pub mod error;
pub mod event;
pub mod llm;
pub mod logging;
pub mod runtime;
pub mod tools;
pub mod types;

// Workflow runtime — opt-in via the `workflow` feature.
#[cfg(feature = "workflow")]
pub mod context;
#[cfg(feature = "workflow")]
pub mod workflow;

/// Re-export of context strategies for backward compatibility.
#[cfg(feature = "workflow")]
pub use context::strategies as context_strategies;
/// Re-export of retry submodule for backward compatibility.
pub use runtime::retry;
/// Re-export of timeout submodule for backward compatibility.
pub use runtime::timeout;
/// Re-export of `tools` module under the older `tool` name for backward compatibility.
pub use tools as tool;
/// Re-export of step types at the crate root for backward compatibility.
#[cfg(feature = "workflow")]
pub use workflow::step;
/// Re-export of step implementations for backward compatibility.
#[cfg(feature = "workflow")]
pub use workflow::steps as step_impls;

// Re-exports for convenience
pub use agent::{Agent, AgentConfig};
pub use config::{
    LlamaConfig, LlmConfig, LoggingConfig, OpenAIConfig, RetryConfig, RuntimeConfig,
    TimeoutConfigSettings, WorkflowConfig,
};
#[cfg(feature = "workflow")]
pub use context::{
    ContextError, ContextManager, MergeStrategy, NoOpManager, WorkflowContext, WorkflowMetadata,
};
#[cfg(feature = "workflow")]
pub use context_strategies::{
    MessageTypeManager, SlidingWindowManager, SummarizationManager, TokenBudgetManager,
};
pub use error::{
    AgentError, AgentErrorCode, ConfigError, ConfigErrorCode, LlmError, LlmErrorCode, RuntimeError,
    ToolError, ToolErrorCode, WorkflowError, WorkflowErrorCode,
};
pub use event::{ComponentStatus, Event, EventScope, EventStream, EventType};
pub use llm::{ChatMessage, ChatRequest, ChatResponse, LlmClient, Role};
pub use logging::FileLogger;
pub use retry::RetryPolicy;
#[cfg(feature = "workflow")]
pub use runtime::Runtime;
#[cfg(feature = "workflow")]
pub use step::{ExecutionContext, Step, StepError, StepInput, StepOutput, StepResult, StepType};
pub use timeout::{with_timeout, TimeoutConfig};
pub use tools::{
    McpClient, McpTool, McpToolInfo, NativeTool, Tool, ToolCallTracker, ToolLoopDetectionConfig,
    ToolRegistry,
};
pub use types::*;
#[cfg(feature = "workflow")]
pub use workflow::steps::{AgentStep, ConditionalStep, SubWorkflowStep, TransformStep};
#[cfg(feature = "workflow")]
pub use workflow::{Workflow, WorkflowBuilder, WorkflowState};

// Prelude module for convenient imports in tests and examples
pub mod prelude {
    pub use crate::agent::{Agent, AgentConfig};
    pub use crate::event::{ComponentStatus, Event, EventScope, EventStream, EventType};
    pub use crate::llm::{ChatMessage, ChatRequest, ChatResponse, LlmClient, Role};
    pub use crate::tools::{NativeTool, Tool, ToolRegistry};
    pub use crate::types::{
        AgentInput, AgentOutput, ToolError as TypesToolError, ToolResult, ToolStatus,
    };
    #[cfg(feature = "workflow")]
    pub use crate::workflow::steps::{AgentStep, ConditionalStep, SubWorkflowStep, TransformStep};
    #[cfg(feature = "workflow")]
    pub use crate::workflow::Workflow;

    #[cfg(test)]
    pub use crate::llm::{MockLlmClient, MockResponse, MockToolCall};

    #[cfg(not(test))]
    pub use crate::llm::{MockLlmClient, MockResponse, MockToolCall};
}
