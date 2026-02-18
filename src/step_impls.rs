use crate::{
    agent::{Agent, AgentConfig},
    runtime::Runtime,
    step::{Step, StepError, StepInput, StepOutput, StepOutputMetadata, StepResult, StepType},
    workflow::Workflow,
};
use async_trait::async_trait;

#[cfg(test)]
#[path = "step_impls_test.rs"]
mod step_impls_test;

/// A step that executes an agent
pub struct AgentStep {
    agent: Agent,
    name: String,
}

impl AgentStep {
    /// Create a new agent step from an agent configuration
    pub fn new(config: AgentConfig) -> Self {
        let name = config.name.clone();
        Self {
            agent: Agent::new(config),
            name,
        }
    }

    /// Create from an existing Agent
    pub fn from_agent(agent: Agent, name: String) -> Self {
        Self { agent, name }
    }
}

#[async_trait]
impl Step for AgentStep {
    async fn execute_with_context(
        &self,
        input: StepInput,
        ctx: crate::step::ExecutionContext<'_>,
    ) -> StepResult {
        let start = std::time::Instant::now();

        // Extract chat history from workflow context if available
        let chat_history = if let Some(context_arc) = &input.workflow_context {
            let context = context_arc.read().unwrap();
            Some(context.history().to_vec())
        } else {
            None
        };

        // Convert StepInput to AgentInput
        let agent_input = crate::types::AgentInput {
            data: input.data,
            metadata: crate::types::AgentInputMetadata {
                step_index: input.metadata.step_index,
                previous_agent: input.metadata.previous_step.clone(),
            },
            chat_history,
        };

        // Execute agent with event stream
        let result = self
            .agent
            .execute_with_events(agent_input, ctx.event_stream)
            .await
            .map_err(|e| StepError::AgentError(e.to_string()))?;

        // Update workflow context with new messages if it exists
        if let Some(context_arc) = &input.workflow_context {
            if let Some(new_history) = &result.chat_history {
                let mut context = context_arc.write().unwrap();
                context.set_history(new_history.clone());
            }
        }

        Ok(StepOutput {
            data: result.data,
            metadata: StepOutputMetadata {
                step_name: self.name.clone(),
                step_type: StepType::Agent,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn step_type(&self) -> StepType {
        StepType::Agent
    }

    fn description(&self) -> Option<&str> {
        Some(self.agent.config().system_prompt.as_str())
    }
}

/// A step that transforms data using a pure function
pub struct TransformStep {
    name: String,
    transform_fn: Box<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>,
}

impl TransformStep {
    pub fn new<F>(name: String, transform_fn: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        Self {
            name,
            transform_fn: Box::new(transform_fn),
        }
    }
}

#[async_trait]
impl Step for TransformStep {
    async fn execute_with_context(
        &self,
        input: StepInput,
        _ctx: crate::step::ExecutionContext<'_>,
    ) -> StepResult {
        // Use the same logic as execute() - transforms don't need events yet
        self.execute(input).await
    }

    async fn execute(&self, input: StepInput) -> StepResult {
        let start = std::time::Instant::now();

        let output_data = (self.transform_fn)(input.data);

        Ok(StepOutput {
            data: output_data,
            metadata: StepOutputMetadata {
                step_name: self.name.clone(),
                step_type: StepType::Transform,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn step_type(&self) -> StepType {
        StepType::Transform
    }
}

/// A step that conditionally executes one of two branches
pub struct ConditionalStep {
    name: String,
    condition_fn: Box<dyn Fn(&serde_json::Value) -> bool + Send + Sync>,
    true_step: Box<dyn Step>,
    false_step: Box<dyn Step>,
}

impl ConditionalStep {
    pub fn new<F>(
        name: String,
        condition_fn: F,
        true_step: Box<dyn Step>,
        false_step: Box<dyn Step>,
    ) -> Self
    where
        F: Fn(&serde_json::Value) -> bool + Send + Sync + 'static,
    {
        Self {
            name,
            condition_fn: Box::new(condition_fn),
            true_step,
            false_step,
        }
    }
}

#[async_trait]
impl Step for ConditionalStep {
    async fn execute_with_context(
        &self,
        input: StepInput,
        ctx: crate::step::ExecutionContext<'_>,
    ) -> StepResult {
        let start = std::time::Instant::now();

        let condition_result = (self.condition_fn)(&input.data);

        let chosen_step = if condition_result {
            &self.true_step
        } else {
            &self.false_step
        };

        // Execute the chosen branch with context
        let mut result = chosen_step.execute_with_context(input, ctx).await?;

        // Update metadata to reflect this conditional step
        result.metadata.step_name = self.name.clone();
        result.metadata.step_type = StepType::Conditional;
        result.metadata.execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    async fn execute(&self, input: StepInput) -> StepResult {
        let start = std::time::Instant::now();

        let condition_result = (self.condition_fn)(&input.data);

        let chosen_step = if condition_result {
            &self.true_step
        } else {
            &self.false_step
        };

        // Execute the chosen branch
        let mut result = chosen_step.execute(input).await?;

        // Update metadata to reflect this conditional step
        result.metadata.step_name = self.name.clone();
        result.metadata.step_type = StepType::Conditional;
        result.metadata.execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn step_type(&self) -> StepType {
        StepType::Conditional
    }

    fn get_branches(&self) -> Option<(&dyn Step, &dyn Step)> {
        Some((self.true_step.as_ref(), self.false_step.as_ref()))
    }
}

/// A step that executes an entire workflow as a sub-workflow
pub struct SubWorkflowStep {
    name: String,
    workflow_builder: Box<dyn Fn() -> Workflow + Send + Sync>,
}

impl SubWorkflowStep {
    pub fn new<F>(name: String, workflow_builder: F) -> Self
    where
        F: Fn() -> Workflow + Send + Sync + 'static,
    {
        Self {
            name,
            workflow_builder: Box::new(workflow_builder),
        }
    }

    /// Execute the sub-workflow using the provided runtime
    /// This ensures events are emitted to the parent's event stream
    /// and allows sharing parent's chat history context
    pub(crate) fn execute_with_runtime<'a>(
        &'a self,
        input: StepInput,
        runtime: &'a crate::runtime::Runtime,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = StepResult> + Send + 'a>> {
        Box::pin(async move {
            let start = std::time::Instant::now();

            // Build the sub-workflow
            let mut sub_workflow = (self.workflow_builder)();

            // Override initial input with step input
            sub_workflow.initial_input = input.data.clone();

            // Share parent's workflow context if it exists
            // This allows sub-workflow agents to continue the conversation
            if let Some(parent_context) = input.workflow_context {
                sub_workflow.context = Some(parent_context);
            }

            // Execute the sub-workflow with parent context
            let parent_workflow_id = Some(input.metadata.workflow_id.clone());
            let run = runtime
                .execute_with_parent(sub_workflow, parent_workflow_id)
                .await;

            if run.state != crate::workflow::WorkflowState::Completed {
                return Err(StepError::ExecutionFailed(format!(
                    "Sub-workflow failed: {:?}",
                    run.state
                )));
            }

            let output_data = run.final_output.unwrap_or(serde_json::json!({}));

            Ok(StepOutput {
                data: output_data,
                metadata: StepOutputMetadata {
                    step_name: self.name.clone(),
                    step_type: StepType::SubWorkflow,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                },
            })
        })
    }
}

#[async_trait]
impl Step for SubWorkflowStep {
    async fn execute_with_context(
        &self,
        input: StepInput,
        _ctx: crate::step::ExecutionContext<'_>,
    ) -> StepResult {
        // This creates a new runtime - won't share events with parent
        // Use execute_with_runtime() from the parent runtime instead
        let runtime = Runtime::new();
        self.execute_with_runtime(input, &runtime).await
    }

    async fn execute(&self, input: StepInput) -> StepResult {
        // This creates a new runtime - won't share events with parent
        // Use execute_with_runtime() from the parent runtime instead
        let runtime = Runtime::new();
        self.execute_with_runtime(input, &runtime).await
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn step_type(&self) -> StepType {
        StepType::SubWorkflow
    }

    fn description(&self) -> Option<&str> {
        Some("Executes a nested workflow")
    }

    fn get_sub_workflow(&self) -> Option<crate::workflow::Workflow> {
        Some((self.workflow_builder)())
    }
}
