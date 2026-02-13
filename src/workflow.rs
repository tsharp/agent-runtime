use serde::{Deserialize, Serialize};
use crate::types::JsonValue;
use crate::step::{Step, StepType};

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
    
    /// Generate a Mermaid flowchart diagram of this workflow
    pub fn to_mermaid(&self) -> String {
        let mut diagram = String::from("flowchart TD\n");
        
        // Start node
        diagram.push_str("    Start([Start])\n");
        
        // Connect start to first step
        if !self.steps.is_empty() {
            diagram.push_str("    Start --> Step0\n");
        }
        
        // Generate nodes for each step
        for (i, step) in self.steps.iter().enumerate() {
            let node_id = format!("Step{}", i);
            let step_name = step.name();
            let step_type = step.step_type();
            
            // Choose node shape based on step type
            let node_def = match step_type {
                StepType::Agent => {
                    // Rounded box for agents
                    format!("    {}[\"{}<br/><i>Agent</i>\"]", node_id, step_name)
                }
                StepType::Transform => {
                    // Parallelogram for transforms
                    format!("    {}[/\"{}<br/><i>Transform</i>\"/]", node_id, step_name)
                }
                StepType::Conditional => {
                    // Diamond for conditionals
                    format!("    {}{{\"{}<br/><i>Conditional</i>\"}}", node_id, step_name)
                }
                StepType::SubWorkflow => {
                    // Double-border box for sub-workflows
                    format!("    {}[[\"{}<br/><i>Sub-Workflow</i>\"]]", node_id, step_name)
                }
                StepType::Parallel => {
                    format!("    {}{{{{{{\"{}<br/><i>Parallel</i>\"}}}}}}}}", node_id, step_name)
                }
                StepType::Custom(ref custom_type) => {
                    format!("    {}[\"{}<br/><i>{}</i>\"]", node_id, step_name, custom_type)
                }
            };
            
            diagram.push_str(&node_def);
            diagram.push('\n');
            
            // Connect to next step
            if i < self.steps.len() - 1 {
                diagram.push_str(&format!("    {} --> Step{}\n", node_id, i + 1));
            }
        }
        
        // End node
        if !self.steps.is_empty() {
            let last_step = self.steps.len() - 1;
            diagram.push_str(&format!("    Step{} --> End\n", last_step));
        } else {
            diagram.push_str("    Start --> End\n");
        }
        
        diagram.push_str("    End([End])\n");
        
        // Add styling
        diagram.push_str("\n");
        diagram.push_str("    classDef agentStyle fill:#e1f5ff,stroke:#01579b,stroke-width:2px\n");
        diagram.push_str("    classDef transformStyle fill:#f3e5f5,stroke:#4a148c,stroke-width:2px\n");
        diagram.push_str("    classDef conditionalStyle fill:#fff3e0,stroke:#e65100,stroke-width:2px\n");
        diagram.push_str("    classDef subworkflowStyle fill:#e8f5e9,stroke:#1b5e20,stroke-width:2px\n");
        
        diagram
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
    
    /// Parent workflow ID if this is a sub-workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_workflow_id: Option<String>,
}

impl WorkflowRun {
    /// Generate a Mermaid flowchart with execution results
    pub fn to_mermaid_with_results(&self) -> String {
        let mut diagram = String::from("flowchart TD\n");
        
        // Start node
        let start_style = match self.state {
            WorkflowState::Completed => ":::successStyle",
            WorkflowState::Failed => ":::failureStyle",
            _ => "",
        };
        diagram.push_str(&format!("    Start([Start]){}  \n", start_style));
        
        // Connect start to first step
        if !self.steps.is_empty() {
            diagram.push_str("    Start --> Step0\n");
        }
        
        // Generate nodes for each step
        for (i, step) in self.steps.iter().enumerate() {
            let node_id = format!("Step{}", i);
            let step_name = &step.step_name;
            let step_type = &step.step_type;
            let exec_time = step.execution_time_ms.unwrap_or(0);
            
            // Determine style based on output
            let has_output = step.output.is_some();
            let style_class = if has_output {
                ":::successStyle"
            } else {
                ":::failureStyle"
            };
            
            // Choose node shape based on step type
            let node_def = if step_type.contains("Agent") {
                format!("    {}[\"{}<br/><i>{}</i><br/>{}ms\"]{}", 
                    node_id, step_name, step_type, exec_time, style_class)
            } else if step_type.contains("Transform") {
                format!("    {}[/\"{}<br/><i>{}</i><br/>{}ms\"/]{}", 
                    node_id, step_name, step_type, exec_time, style_class)
            } else if step_type.contains("Conditional") {
                format!("    {}{{\"{}<br/><i>{}</i><br/>{}ms\"}}{}", 
                    node_id, step_name, step_type, exec_time, style_class)
            } else if step_type.contains("SubWorkflow") {
                format!("    {}[[\"{}<br/><i>{}</i><br/>{}ms\"]]{}", 
                    node_id, step_name, step_type, exec_time, style_class)
            } else {
                format!("    {}[\"{}<br/><i>{}</i><br/>{}ms\"]{}", 
                    node_id, step_name, step_type, exec_time, style_class)
            };
            
            diagram.push_str(&node_def);
            diagram.push('\n');
            
            // Connect to next step
            if i < self.steps.len() - 1 {
                diagram.push_str(&format!("    {} --> Step{}\n", node_id, i + 1));
            }
        }
        
        // End node
        if !self.steps.is_empty() {
            let last_step = self.steps.len() - 1;
            diagram.push_str(&format!("    Step{} --> End\n", last_step));
        } else {
            diagram.push_str("    Start --> End\n");
        }
        
        let end_style = match self.state {
            WorkflowState::Completed => ":::successStyle",
            WorkflowState::Failed => ":::failureStyle",
            _ => "",
        };
        diagram.push_str(&format!("    End([End]){}\n", end_style));
        
        // Add styling
        diagram.push_str("\n");
        diagram.push_str("    classDef successStyle fill:#c8e6c9,stroke:#2e7d32,stroke-width:3px\n");
        diagram.push_str("    classDef failureStyle fill:#ffcdd2,stroke:#c62828,stroke-width:3px\n");
        
        diagram
    }
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

