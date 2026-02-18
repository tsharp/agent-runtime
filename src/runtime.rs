use crate::{
    event::{Event, EventStream},
    step::{StepInput, StepInputMetadata, StepType},
    step_impls::SubWorkflowStep,
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

    /// Get a reference to the event stream for subscribing to events
    pub fn event_stream(&self) -> &EventStream {
        &self.event_stream
    }

    /// Execute a workflow and return the run with complete history
    pub async fn execute(&self, workflow: Workflow) -> WorkflowRun {
        self.execute_with_parent(workflow, None).await
    }

    /// Execute a workflow with optional parent workflow context
    pub async fn execute_with_parent(
        &self,
        mut workflow: Workflow,
        parent_workflow_id: Option<String>,
    ) -> WorkflowRun {
        let workflow_id = workflow.id.clone();

        // Emit Workflow::Started event
        self.event_stream.workflow_started(
            &workflow_id,
            serde_json::json!({
                "step_count": workflow.steps.len(),
                "parent_workflow_id": parent_workflow_id,
            }),
        );

        workflow.state = WorkflowState::Running;

        let mut run = WorkflowRun {
            workflow_id: workflow_id.clone(),
            state: WorkflowState::Running,
            steps: Vec::new(),
            final_output: None,
            parent_workflow_id: parent_workflow_id.clone(),
        };

        let mut current_data = workflow.initial_input.clone();

        // Execute each step in sequence
        for (step_index, step) in workflow.steps.iter().enumerate() {
            let step_name = step.name().to_string();
            let step_type_enum = step.step_type();
            let step_type = format!("{:?}", step_type_enum);

            // Emit WorkflowStep::Started event
            self.event_stream.step_started(
                &workflow_id,
                step_index,
                serde_json::json!({
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
                workflow_context: workflow.context.clone(),
            };

            // Execute step - special handling for SubWorkflowStep
            let result = if step_type_enum == StepType::SubWorkflow {
                // Cast to SubWorkflowStep and execute with this runtime
                // to share the event stream
                let sub_step = unsafe {
                    // SAFETY: We just checked step_type is SubWorkflow
                    let ptr =
                        step.as_ref() as *const dyn crate::step::Step as *const SubWorkflowStep;
                    &*ptr
                };
                sub_step.execute_with_runtime(input.clone(), self).await
            } else {
                // Execute with event stream context
                let ctx = crate::step::ExecutionContext::with_event_stream(&self.event_stream);
                step.execute_with_context(input.clone(), ctx).await
            };

            match result {
                Ok(output) => {
                    // Emit WorkflowStep::Completed event
                    self.event_stream.step_completed(
                        &workflow_id,
                        step_index,
                        serde_json::json!({
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
                    // Emit WorkflowStep::Failed event
                    self.event_stream.step_failed(
                        &workflow_id,
                        step_index,
                        &e.to_string(),
                        serde_json::json!({
                            "step_name": &step_name,
                        }),
                    );

                    // Emit Workflow::Failed event
                    self.event_stream.workflow_failed(
                        &workflow_id,
                        &e.to_string(),
                        serde_json::json!({
                            "failed_step": step_index,
                            "failed_step_name": &step_name,
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

        self.event_stream.workflow_completed(
            &workflow_id,
            serde_json::json!({
                "steps_completed": run.steps.len(),
            }),
        );

        run
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
