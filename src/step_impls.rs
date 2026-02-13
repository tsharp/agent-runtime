use async_trait::async_trait;
use crate::{
    agent::{Agent, AgentConfig},
    step::{Step, StepInput, StepOutput, StepResult, StepType, StepError, StepOutputMetadata},
};

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
    async fn execute(&self, input: StepInput) -> StepResult {
        let start = std::time::Instant::now();
        
        // Convert StepInput to AgentInput
        let agent_input = crate::types::AgentInput {
            data: input.data,
            metadata: crate::types::AgentInputMetadata {
                step_index: input.metadata.step_index,
                previous_agent: input.metadata.previous_step.clone(),
            },
        };
        
        // Execute agent
        let result = self.agent.execute(agent_input).await
            .map_err(|e| StepError::AgentError(e.to_string()))?;
        
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
}
