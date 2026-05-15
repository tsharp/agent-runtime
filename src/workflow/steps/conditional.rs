use crate::workflow::step::{ExecutionContext, Step, StepInput, StepResult, StepType};
use async_trait::async_trait;

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
        ctx: ExecutionContext<'_>,
    ) -> StepResult {
        let start = std::time::Instant::now();

        let condition_result = (self.condition_fn)(&input.data);

        let chosen_step = if condition_result {
            &self.true_step
        } else {
            &self.false_step
        };

        let mut result = chosen_step.execute_with_context(input, ctx).await?;

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

        let mut result = chosen_step.execute(input).await?;

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
