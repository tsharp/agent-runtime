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
    
    /// Generate a Mermaid flowchart diagram of this workflow with full expansion
    pub fn to_mermaid(&self) -> String {
        let mut diagram = String::from("flowchart TD\n");
        let mut node_counter = 0;
        
        // Start node
        diagram.push_str("    Start([Start])\n");
        
        if self.steps.is_empty() {
            diagram.push_str("    Start --> End\n");
        } else {
            // Connect start to first step
            let first_node = format!("N{}", node_counter);
            diagram.push_str(&format!("    Start --> {}\n", first_node));
            
            // Generate recursive structure
            let last_node = self.generate_mermaid_steps(
                &mut diagram,
                &mut node_counter,
                &first_node,
                0,
            );
            
            // Connect last step to end
            diagram.push_str(&format!("    {} --> End\n", last_node));
        }
        
        diagram.push_str("    End([End])\n");
        
        // Add styling
        diagram.push_str("\n");
        diagram.push_str("    classDef agentStyle fill:#e1f5ff,stroke:#01579b,stroke-width:2px\n");
        diagram.push_str("    classDef transformStyle fill:#f3e5f5,stroke:#4a148c,stroke-width:2px\n");
        diagram.push_str("    classDef conditionalStyle fill:#fff3e0,stroke:#e65100,stroke-width:2px\n");
        diagram.push_str("    classDef subworkflowStyle fill:#e8f5e9,stroke:#1b5e20,stroke-width:2px\n");
        diagram.push_str("    classDef convergeStyle fill:#f5f5f5,stroke:#757575,stroke-width:1px\n");
        
        diagram
    }
    
    /// Recursive helper to generate mermaid steps
    fn generate_mermaid_steps(
        &self,
        diagram: &mut String,
        node_counter: &mut usize,
        entry_node: &str,
        step_index: usize,
    ) -> String {
        if step_index >= self.steps.len() {
            return entry_node.to_string();
        }
        
        let step = &self.steps[step_index];
        let step_type = step.step_type();
        
        match step_type {
            StepType::Conditional => {
                // Get branches
                if let Some((then_step, else_step)) = step.get_branches() {
                    let conditional_node = entry_node;
                    
                    // Create conditional diamond
                    let step_name = step.name();
                    diagram.push_str(&format!(
                        "    {}{{{{\"{}\"}}}}:::conditionalStyle\n",
                        conditional_node, step_name
                    ));
                    
                    // Then branch
                    *node_counter += 1;
                    let then_node = format!("N{}", node_counter);
                    let then_exit_node;
                    
                    // Check if then branch is a SubWorkflow
                    if then_step.step_type() == StepType::SubWorkflow {
                        if let Some(sub_wf) = then_step.get_sub_workflow() {
                            // Generate subworkflow and get (entry, exit)
                            let (_entry, exit) = self.generate_subworkflow_inline(
                                diagram, 
                                node_counter, 
                                &then_node, 
                                sub_wf, 
                                then_step.name()
                            );
                            then_exit_node = exit;
                        } else {
                            self.generate_step_node(diagram, &then_node, then_step);
                            then_exit_node = then_node.clone();
                        }
                    } else {
                        self.generate_step_node(diagram, &then_node, then_step);
                        then_exit_node = then_node.clone();
                    }
                    
                    diagram.push_str(&format!(
                        "    {} -->|\"✓ TRUE\"| {}\n",
                        conditional_node, then_node
                    ));
                    
                    // Else branch
                    *node_counter += 1;
                    let else_node = format!("N{}", node_counter);
                    let else_exit_node;
                    
                    // Check if else branch is a SubWorkflow
                    if else_step.step_type() == StepType::SubWorkflow {
                        if let Some(sub_wf) = else_step.get_sub_workflow() {
                            let (_entry, exit) = self.generate_subworkflow_inline(
                                diagram, 
                                node_counter, 
                                &else_node, 
                                sub_wf, 
                                else_step.name()
                            );
                            else_exit_node = exit;
                        } else {
                            self.generate_step_node(diagram, &else_node, else_step);
                            else_exit_node = else_node.clone();
                        }
                    } else {
                        self.generate_step_node(diagram, &else_node, else_step);
                        else_exit_node = else_node.clone();
                    }
                    
                    diagram.push_str(&format!(
                        "    {} -->|\"✗ FALSE\"| {}\n",
                        conditional_node, else_node
                    ));
                    
                    // Convergence point - use EXIT nodes from branches
                    *node_counter += 1;
                    let converge_node = format!("N{}", node_counter);
                    diagram.push_str(&format!(
                        "    {}(( )):::convergeStyle\n",
                        converge_node
                    ));
                    diagram.push_str(&format!("    {} --> {}\n", then_exit_node, converge_node));
                    diagram.push_str(&format!("    {} --> {}\n", else_exit_node, converge_node));
                    
                    // Continue with next step
                    if step_index + 1 < self.steps.len() {
                        // Check if next step is a subworkflow - if so, DON'T create intermediate node
                        let next_step = &self.steps[step_index + 1];
                        if next_step.step_type() == StepType::SubWorkflow {
                            // Pass converge_node directly as entry for subworkflow
                            return self.generate_mermaid_steps(diagram, node_counter, &converge_node, step_index + 1);
                        } else {
                            *node_counter += 1;
                            let next_node = format!("N{}", node_counter);
                            diagram.push_str(&format!("    {} --> {}\n", converge_node, next_node));
                            return self.generate_mermaid_steps(diagram, node_counter, &next_node, step_index + 1);
                        }
                    } else {
                        return converge_node;
                    }
                }
            }
            StepType::SubWorkflow => {
                // Get sub-workflow and expand it
                if let Some(sub_wf) = step.get_sub_workflow() {
                    let sub_start_node = entry_node;
                    
                    // Generate subworkflow and get its exit node
                    let (_entry, sub_exit_node) = self.generate_subworkflow_inline(
                        diagram, 
                        node_counter, 
                        sub_start_node, 
                        sub_wf,
                        step.name()
                    );
                    
                    // Continue with next step from subworkflow exit
                    return self.generate_mermaid_steps(diagram, node_counter, &sub_exit_node, step_index + 1);
                }
            }
            _ => {
                // Regular step (Agent, Transform, etc.)
                let current_node = entry_node;
                self.generate_step_node(diagram, current_node, step.as_ref());
                
                // Continue with next step if there is one
                if step_index + 1 < self.steps.len() {
                    // Check if next step is subworkflow
                    let next_step = &self.steps[step_index + 1];
                    if next_step.step_type() == StepType::SubWorkflow {
                        // Pass current_node directly as entry for subworkflow
                        return self.generate_mermaid_steps(diagram, node_counter, current_node, step_index + 1);
                    } else {
                        *node_counter += 1;
                        let next_node = format!("N{}", node_counter);
                        diagram.push_str(&format!("    {} --> {}\n", current_node, next_node));
                        return self.generate_mermaid_steps(diagram, node_counter, &next_node, step_index + 1);
                    }
                } else {
                    // This is the last step
                    return current_node.to_string();
                }
            }
        }
        
        entry_node.to_string()
    }
    
    /// Generate a subworkflow inline as a subgraph
    /// Returns (entry_node, exit_node) tuple
    fn generate_subworkflow_inline(
        &self,
        diagram: &mut String,
        node_counter: &mut usize,
        entry_point: &str,
        sub_wf: Workflow,
        step_name: &str,
    ) -> (String, String) {
        let subgraph_id = *node_counter;
        
        // Create sub-workflow container
        diagram.push_str(&format!(
            "    subgraph SUB{}[\"📦 {}\"]\n",
            subgraph_id, step_name
        ));
        
        *node_counter += 1;
        let sub_entry = format!("N{}", node_counter);
        diagram.push_str(&format!("        {}([Start])\n", sub_entry));
        
        // Generate sub-workflow steps recursively
        let sub_exit = sub_wf.generate_mermaid_steps_in_subgraph(
            diagram,
            node_counter,
            &sub_entry,
            0,
        );
        
        *node_counter += 1;
        let sub_end = format!("N{}", node_counter);
        diagram.push_str(&format!("        {}([End])\n", sub_end));
        diagram.push_str(&format!("        {} --> {}\n", sub_exit, sub_end));
        
        diagram.push_str("    end\n");
        
        // Connect entry point to subworkflow start
        diagram.push_str(&format!("    {} --> {}\n", entry_point, sub_entry));
        
        // Return both entry point (for conditional connection) and exit node (for continuation)
        (entry_point.to_string(), sub_end)
    }
    
    /// Generate steps within a subgraph (different indentation)
    fn generate_mermaid_steps_in_subgraph(
        &self,
        diagram: &mut String,
        node_counter: &mut usize,
        entry_node: &str,
        step_index: usize,
    ) -> String {
        if step_index >= self.steps.len() {
            return entry_node.to_string();
        }
        
        let step = &self.steps[step_index];
        let step_type = step.step_type();
        
        match step_type {
            StepType::Conditional => {
                if let Some((then_step, else_step)) = step.get_branches() {
                    let conditional_node = entry_node;
                    
                    diagram.push_str(&format!(
                        "        {}{{{{\"{}\"}}}}:::conditionalStyle\n",
                        conditional_node, step.name()
                    ));
                    
                    *node_counter += 1;
                    let then_node = format!("N{}", node_counter);
                    self.generate_step_node_indented(diagram, &then_node, then_step);
                    diagram.push_str(&format!(
                        "        {} -->|\"✓\"| {}\n",
                        conditional_node, then_node
                    ));
                    
                    *node_counter += 1;
                    let else_node = format!("N{}", node_counter);
                    self.generate_step_node_indented(diagram, &else_node, else_step);
                    diagram.push_str(&format!(
                        "        {} -->|\"✗\"| {}\n",
                        conditional_node, else_node
                    ));
                    
                    *node_counter += 1;
                    let converge_node = format!("N{}", node_counter);
                    diagram.push_str(&format!("        {}(( )):::convergeStyle\n", converge_node));
                    diagram.push_str(&format!("        {} --> {}\n", then_node, converge_node));
                    diagram.push_str(&format!("        {} --> {}\n", else_node, converge_node));
                    
                    *node_counter += 1;
                    let next_node = format!("N{}", node_counter);
                    diagram.push_str(&format!("        {} --> {}\n", converge_node, next_node));
                    
                    return self.generate_mermaid_steps_in_subgraph(diagram, node_counter, &next_node, step_index + 1);
                }
            }
            StepType::SubWorkflow => {
                // Nested subworkflow within a subworkflow
                if let Some(nested_wf) = step.get_sub_workflow() {
                    let nested_entry = entry_node;
                    
                    let nested_id = *node_counter;
                    diagram.push_str(&format!(
                        "        subgraph NESTED{}[\"📦 {}\"]\n",
                        nested_id, step.name()
                    ));
                    
                    *node_counter += 1;
                    let nested_start = format!("N{}", node_counter);
                    diagram.push_str(&format!("            {}([Start])\n", nested_start));
                    
                    let nested_exit = nested_wf.generate_mermaid_steps_in_nested_subgraph(
                        diagram,
                        node_counter,
                        &nested_start,
                        0,
                    );
                    
                    *node_counter += 1;
                    let nested_end = format!("N{}", node_counter);
                    diagram.push_str(&format!("            {}([End])\n", nested_end));
                    diagram.push_str(&format!("            {} --> {}\n", nested_exit, nested_end));
                    diagram.push_str("        end\n");
                    
                    diagram.push_str(&format!("        {} --> {}\n", nested_entry, nested_start));
                    
                    *node_counter += 1;
                    let next_node = format!("N{}", node_counter);
                    diagram.push_str(&format!("        {} --> {}\n", nested_end, next_node));
                    
                    return self.generate_mermaid_steps_in_subgraph(diagram, node_counter, &next_node, step_index + 1);
                }
            }
            _ => {
                let current_node = entry_node;
                self.generate_step_node_indented(diagram, current_node, step.as_ref());
                
                *node_counter += 1;
                let next_node = format!("N{}", node_counter);
                diagram.push_str(&format!("        {} --> {}\n", current_node, next_node));
                
                return self.generate_mermaid_steps_in_subgraph(diagram, node_counter, &next_node, step_index + 1);
            }
        }
        
        entry_node.to_string()
    }
    
    /// Generate steps within a nested subgraph (triple indentation)
    fn generate_mermaid_steps_in_nested_subgraph(
        &self,
        diagram: &mut String,
        node_counter: &mut usize,
        entry_node: &str,
        step_index: usize,
    ) -> String {
        if step_index >= self.steps.len() {
            return entry_node.to_string();
        }
        
        let step = &self.steps[step_index];
        let current_node = entry_node;
        
        // For simplicity at 3rd level, just show step names without further nesting
        let step_name = step.name();
        let step_type = step.step_type();
        
        let (node_def, style) = match step_type {
            StepType::Agent => (
                format!("            {}[\"{}\"]", current_node, step_name),
                ":::agentStyle"
            ),
            StepType::Transform => (
                format!("            {}[/\"{}\"/]", current_node, step_name),
                ":::transformStyle"
            ),
            StepType::Conditional => (
                format!("            {}{{\"{}\"}}", current_node, step_name),
                ":::conditionalStyle"
            ),
            _ => (
                format!("            {}[\"{}\"]", current_node, step_name),
                ""
            ),
        };
        
        diagram.push_str(&format!("{}{}\n", node_def, style));
        
        *node_counter += 1;
        let next_node = format!("N{}", node_counter);
        diagram.push_str(&format!("            {} --> {}\n", current_node, next_node));
        
        self.generate_mermaid_steps_in_nested_subgraph(diagram, node_counter, &next_node, step_index + 1)
    }
    
    /// Generate a single step node (normal indentation)
    fn generate_step_node(&self, diagram: &mut String, node_id: &str, step: &dyn Step) {
        let step_name = step.name();
        let step_type = step.step_type();
        
        let (node_def, style) = match step_type {
            StepType::Agent => (
                format!("    {}[\"{}\"]", node_id, step_name),
                ":::agentStyle"
            ),
            StepType::Transform => (
                format!("    {}[/\"{}\"/]", node_id, step_name),
                ":::transformStyle"
            ),
            StepType::SubWorkflow => (
                format!("    {}[[\"{}\"]", node_id, step_name),
                ":::subworkflowStyle"
            ),
            _ => (
                format!("    {}[\"{}\"]", node_id, step_name),
                ""
            ),
        };
        
        diagram.push_str(&format!("{}{}\n", node_def, style));
    }
    
    /// Generate a single step node (indented for subgraph)
    fn generate_step_node_indented(&self, diagram: &mut String, node_id: &str, step: &dyn Step) {
        let step_name = step.name();
        let step_type = step.step_type();
        
        let (node_def, style) = match step_type {
            StepType::Agent => (
                format!("        {}[\"{}\"]", node_id, step_name),
                ":::agentStyle"
            ),
            StepType::Transform => (
                format!("        {}[/\"{}\"/]", node_id, step_name),
                ":::transformStyle"
            ),
            _ => (
                format!("        {}[\"{}\"]", node_id, step_name),
                ""
            ),
        };
        
        diagram.push_str(&format!("{}{}\n", node_def, style));
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

