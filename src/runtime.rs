use crate::{
    event::{Event, EventStream, EventType},
    step::{StepInput, StepInputMetadata},
    workflow::{Workflow, WorkflowRun, WorkflowState, WorkflowStepRecord},
};

/// Runtime for executing workflows
pub struct Runtime {
    event_stream: EventStream,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            event_stream: EventStream::new(),
        }
    }
    
    /// Execute a workflow and return the run with complete history
    pub async fn execute(&self, mut workflow: Workflow) -> WorkflowRun {
        let workflow_id = workflow.id.clone();
        
        // Emit workflow started event
        self.event_stream.append(
            EventType::WorkflowStarted,
            workflow_id.clone(),
            serde_json::json!({
                "step_count": workflow.steps.len(),
            }),
        );
        
        workflow.state = WorkflowState::Running;
        
        let mut run = WorkflowRun {
            workflow_id: workflow_id.clone(),
            state: WorkflowState::Running,
            steps: Vec::new(),
            final_output: None,
        };
        
        let mut current_data = workflow.initial_input.clone();
        
        // Execute each step in sequence
        for (step_index, step) in workflow.steps.iter().enumerate() {
            let step_name = step.name().to_string();
            let step_type = format!("{:?}", step.step_type());
            
            // Emit step started event
            self.event_stream.append(
                EventType::WorkflowStepStarted,
                workflow_id.clone(),
                serde_json::json!({
                    "step_index": step_index,
                    "step_name": &step_name,
                    "step_type": &step_type,
                }),
            );
            
            // Create step input
            let input = StepInput {
                data: current_data.clone(),
                metadata: StepInputMetadata {
                    step_index,
                    previous_step: if step_index > 0 {
                        Some(workflow.steps[step_index - 1].name().to_string())
                    } else {
                        None
                    },
                    workflow_id: workflow_id.clone(),
                },
            };
            
            // Execute step
            match step.execute(input.clone()).await {
                Ok(output) => {
                    // Emit step completed
                    self.event_stream.append(
                        EventType::WorkflowStepCompleted,
                        workflow_id.clone(),
                        serde_json::json!({
                            "step_index": step_index,
                            "step_name": &step_name,
                            "execution_time_ms": output.metadata.execution_time_ms,
                        }),
                    );
                    
                    // Record step
                    run.steps.push(WorkflowStepRecord {
                        step_index,
                        step_name: step_name.clone(),
                        step_type: step_type.clone(),
                        input: input.data,
                        output: Some(output.data.clone()),
                        execution_time_ms: Some(output.metadata.execution_time_ms),
                    });
                    
                    // Pass output to next step
                    current_data = output.data;
                }
                Err(e) => {
                    // Emit step failed
                    self.event_stream.append(
                        EventType::AgentFailed, // TODO: Add StepFailed event type
                        workflow_id.clone(),
                        serde_json::json!({
                            "step_name": &step_name,
                            "error": e.to_string(),
                        }),
                    );
                    
                    // Emit workflow failed
                    self.event_stream.append(
                        EventType::WorkflowFailed,
                        workflow_id.clone(),
                        serde_json::json!({
                            "error": e.to_string(),
                            "failed_step": step_index,
                        }),
                    );
                    
                    workflow.state = WorkflowState::Failed;
                    run.state = WorkflowState::Failed;
                    return run;
                }
            }
        }
        
        // Workflow completed successfully
        run.final_output = Some(current_data);
        run.state = WorkflowState::Completed;
        workflow.state = WorkflowState::Completed;
        
        self.event_stream.append(
            EventType::WorkflowCompleted,
            workflow_id.clone(),
            serde_json::json!({
                "steps_completed": run.steps.len(),
            }),
        );
        
        run
    }
    
    /// Get the event stream for observability
    pub fn event_stream(&self) -> &EventStream {
        &self.event_stream
    }
    
    /// Get events from a specific offset (for replay)
    pub fn events_from_offset(&self, offset: u64) -> Vec<Event> {
        self.event_stream.from_offset(offset)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

