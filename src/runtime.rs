use crate::{
    agent::Agent,
    event::{Event, EventStream, EventType},
    types::{AgentInput, AgentInputMetadata},
    workflow::{Workflow, WorkflowRun, WorkflowState, WorkflowStep},
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
                "agent_count": workflow.agents.len(),
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
        
        // Execute each agent in sequence
        for (step_index, agent_config) in workflow.agents.iter().enumerate() {
            let agent = Agent::new(agent_config.clone());
            
            // Emit step started event
            self.event_stream.append(
                EventType::WorkflowStepStarted,
                workflow_id.clone(),
                serde_json::json!({
                    "step_index": step_index,
                    "agent_name": agent.name(),
                }),
            );
            
            // Create agent input
            let input = AgentInput {
                data: current_data.clone(),
                metadata: AgentInputMetadata {
                    step_index,
                    previous_agent: if step_index > 0 {
                        Some(workflow.agents[step_index - 1].name.clone())
                    } else {
                        None
                    },
                },
            };
            
            // Emit agent initialized
            self.event_stream.append(
                EventType::AgentInitialized,
                workflow_id.clone(),
                serde_json::json!({
                    "agent_name": agent.name(),
                    "step_index": step_index,
                }),
            );
            
            // Execute agent
            let step_start = std::time::Instant::now();
            match agent.execute(input.clone()).await {
                Ok(output) => {
                    let execution_time = step_start.elapsed().as_millis() as u64;
                    
                    // Emit agent completed
                    self.event_stream.append(
                        EventType::AgentCompleted,
                        workflow_id.clone(),
                        serde_json::json!({
                            "agent_name": agent.name(),
                            "execution_time_ms": execution_time,
                        }),
                    );
                    
                    // Record step
                    run.steps.push(WorkflowStep {
                        step_index,
                        agent_name: agent.name().to_string(),
                        input: input.data,
                        output: Some(output.data.clone()),
                        execution_time_ms: Some(execution_time),
                    });
                    
                    // Pass output to next agent
                    current_data = output.data;
                    
                    // Emit step completed
                    self.event_stream.append(
                        EventType::WorkflowStepCompleted,
                        workflow_id.clone(),
                        serde_json::json!({
                            "step_index": step_index,
                            "agent_name": agent.name(),
                        }),
                    );
                }
                Err(e) => {
                    // Emit agent failed
                    self.event_stream.append(
                        EventType::AgentFailed,
                        workflow_id.clone(),
                        serde_json::json!({
                            "agent_name": agent.name(),
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
