use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::tool::Tool;
use crate::types::{AgentInput, AgentResult, AgentOutput, AgentOutputMetadata};

/// Agent configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub system_prompt: String,
    
    #[serde(skip)]
    pub tools: Vec<Arc<dyn Tool>>,
}

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConfig")
            .field("name", &self.name)
            .field("system_prompt", &self.system_prompt)
            .field("tools", &format!("{} tools", self.tools.len()))
            .finish()
    }
}

impl AgentConfig {
    pub fn builder(name: impl Into<String>) -> AgentConfigBuilder {
        AgentConfigBuilder {
            name: name.into(),
            system_prompt: String::new(),
            tools: Vec::new(),
        }
    }
}

/// Builder for AgentConfig
pub struct AgentConfigBuilder {
    name: String,
    system_prompt: String,
    tools: Vec<Arc<dyn Tool>>,
}

impl AgentConfigBuilder {
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }
    
    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }
    
    pub fn tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools = tools;
        self
    }
    
    pub fn build(self) -> AgentConfig {
        AgentConfig {
            name: self.name,
            system_prompt: self.system_prompt,
            tools: self.tools,
        }
    }
}

/// Agent execution unit
pub struct Agent {
    config: AgentConfig,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }
    
    pub fn name(&self) -> &str {
        &self.config.name
    }
    
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    /// Execute the agent with the given input
    /// Note: This is a simplified mock implementation
    /// Real implementation would call LLM APIs
    pub async fn execute(&self, input: AgentInput) -> AgentResult {
        let start = std::time::Instant::now();
        
        // Mock execution: for now, just pass through the input
        // In real implementation, this would:
        // 1. Build context with system prompt + input
        // 2. Call LLM API
        // 3. Handle tool calls
        // 4. Return final output
        
        let output_data = serde_json::json!({
            "agent": self.config.name,
            "processed": input.data,
            "system_prompt": self.config.system_prompt,
        });
        
        Ok(AgentOutput {
            data: output_data,
            metadata: AgentOutputMetadata {
                agent_name: self.config.name.clone(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                tool_calls_count: 0,
            },
        })
    }
}
