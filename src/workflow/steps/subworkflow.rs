use crate::runtime::Runtime;
use crate::workflow::step::{
    ExecutionContext, Step, StepError, StepInput, StepOutput, StepOutputMetadata, StepResult,
    StepType,
};
use crate::workflow::Workflow;
use async_trait::async_trait;

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
        runtime: &'a Runtime,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = StepResult> + Send + 'a>> {
        Box::pin(async move {
            let start = std::time::Instant::now();

            let mut sub_workflow = (self.workflow_builder)();

            sub_workflow.initial_input = input.data.clone();

            if let Some(parent_context) = input.workflow_context {
                sub_workflow.context = Some(parent_context);
            }

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
        _ctx: ExecutionContext<'_>,
    ) -> StepResult {
        let runtime = Runtime::new();
        self.execute_with_runtime(input, &runtime).await
    }

    async fn execute(&self, input: StepInput) -> StepResult {
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

    fn get_sub_workflow(&self) -> Option<Workflow> {
        Some((self.workflow_builder)())
    }
}
