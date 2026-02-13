use serde::{Deserialize, Serialize};
use crate::agent::AgentConfig;
use crate::types::{WorkflowId, JsonValue};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: WorkflowId,
    pub agents: Vec<AgentConfig>,
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
    agents: Vec<AgentConfig>,
    initial_input: Option<JsonValue>,
}

impl WorkflowBuilder {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            initial_input: None,
        }
    }
    
    pub fn agent(mut self, agent: AgentConfig) -> Self {
        self.agents.push(agent);
        self
    }
    
    pub fn initial_input(mut self, input: JsonValue) -> Self {
        self.initial_input = Some(input);
        self
    }
    
    pub fn build(self) -> Workflow {
        Workflow {
            id: format!("wf_{}", uuid::Uuid::new_v4()),
            agents: self.agents,
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
    pub workflow_id: WorkflowId,
    pub state: WorkflowState,
    pub steps: Vec<WorkflowStep>,
    pub final_output: Option<JsonValue>,
}

/// A single step in workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_index: usize,
    pub agent_name: String,
    pub input: JsonValue,
    pub output: Option<JsonValue>,
    pub execution_time_ms: Option<u64>,
}
