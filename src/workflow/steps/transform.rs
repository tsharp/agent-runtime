use crate::workflow::step::{
    ExecutionContext, Step, StepInput, StepOutput, StepOutputMetadata, StepResult, StepType,
};
use async_trait::async_trait;

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
        _ctx: ExecutionContext<'_>,
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
