use crate::agent::{Agent, AgentConfig};
use crate::workflow::step::{
    ExecutionContext, Step, StepError, StepInput, StepOutput, StepOutputMetadata, StepResult,
    StepType,
};
use async_trait::async_trait;

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
        ctx: ExecutionContext<'_>,
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
