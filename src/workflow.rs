use serde::{Deserialize, Serialize};
use crate::types::JsonValue;
use crate::step::Step;

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Workflow definition
pub struct Workflow {
    pub id: String,
    pub steps: Vec<Box<dyn Step>>,
    pub initial_input: JsonValue,
    pub state: WorkflowState,
}

impl Workflow {
    pub fn builder() -> WorkflowBuilder {
        WorkflowBuilder::new()
    }
}

/// Builder for Workflow
pub struct WorkflowBuilder {
    steps: Vec<Box<dyn Step>>,
    initial_input: Option<JsonValue>,
}

impl WorkflowBuilder {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            initial_input: None,
        }
    }
    
    /// Add a step to the workflow
    pub fn step(mut self, step: Box<dyn Step>) -> Self {
        self.steps.push(step);
        self
    }
    
    /// Set the initial input
    pub fn initial_input(mut self, input: JsonValue) -> Self {
        self.initial_input = Some(input);
        self
    }
    
    pub fn build(self) -> Workflow {
        Workflow {
            id: format!("wf_{}", uuid::Uuid::new_v4()),
            steps: self.steps,
            initial_input: self.initial_input.unwrap_or(serde_json::json!({})),
            state: WorkflowState::Pending,
        }
    }
}

impl Default for WorkflowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A workflow execution run with complete history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub workflow_id: String,
    pub state: WorkflowState,
    pub steps: Vec<WorkflowStepRecord>,
    pub final_output: Option<JsonValue>,
}

/// A single step record in workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepRecord {
    pub step_index: usize,
    pub step_name: String,
    pub step_type: String,
    pub input: JsonValue,
    pub output: Option<JsonValue>,
    pub execution_time_ms: Option<u64>,
}

