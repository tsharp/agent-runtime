use crate::event::EventStream;
use crate::types::JsonValue;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Types of steps that can be in a workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    Agent,
    Transform,
    Conditional,
    Parallel,
    SubWorkflow,
    Custom(String),
}

/// Input data passed to a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInput {
    pub data: JsonValue,
    pub metadata: StepInputMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepInputMetadata {
    pub step_index: usize,
    pub previous_step: Option<String>,
    pub workflow_id: String,
}

/// Output data produced by a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub data: JsonValue,
    pub metadata: StepOutputMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutputMetadata {
    pub step_name: String,
    pub step_type: StepType,
    pub execution_time_ms: u64,
}

/// Result type for step execution
pub type StepResult = Result<StepOutput, StepError>;

/// Errors that can occur during step execution
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum StepError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Agent error: {0}")]
    AgentError(String),

    #[error("Step not found: {0}")]
    StepNotFound(String),
}

/// Execution context passed to steps
pub struct ExecutionContext<'a> {
    pub event_stream: Option<&'a EventStream>,
}

impl<'a> ExecutionContext<'a> {
    pub fn new() -> Self {
        Self { event_stream: None }
    }

    pub fn with_event_stream(event_stream: &'a EventStream) -> Self {
        Self {
            event_stream: Some(event_stream),
        }
    }
}

/// Step trait - all workflow steps must implement this
#[async_trait]
pub trait Step: Send + Sync {
    /// Execute the step with the given input
    async fn execute(&self, input: StepInput) -> StepResult {
        self.execute_with_context(input, ExecutionContext::new())
            .await
    }

    /// Execute with execution context (for event streaming, etc.)
    async fn execute_with_context(&self, input: StepInput, ctx: ExecutionContext<'_>)
        -> StepResult;

    /// Unique name for this step
    fn name(&self) -> &str;

    /// Type of step
    fn step_type(&self) -> StepType;

    /// Optional: Get a description of what this step does
    fn description(&self) -> Option<&str> {
        None
    }

    /// For conditional steps: get the branches (then, else)
    fn get_branches(&self) -> Option<(&dyn Step, &dyn Step)> {
        None
    }

    /// For sub-workflow steps: get the workflow
    fn get_sub_workflow(&self) -> Option<crate::workflow::Workflow> {
        None
    }
}
