use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::tool::Tool;
use crate::types::{AgentInput, AgentResult, AgentOutput, AgentOutputMetadata, AgentError};
use crate::llm::{ChatClient, ChatMessage, ChatRequest};

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
    llm_client: Option<Arc<dyn ChatClient>>,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self { 
            config,
            llm_client: None,
        }
    }
    
    pub fn with_llm_client(mut self, client: Arc<dyn ChatClient>) -> Self {
        self.llm_client = Some(client);
        self
    }
    
    pub fn name(&self) -> &str {
        &self.config.name
    }
    
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    /// Execute the agent with the given input
    pub async fn execute(&self, input: AgentInput) -> AgentResult {
        let start = std::time::Instant::now();
        
        // If we have an LLM client, use it
        if let Some(client) = &self.llm_client {
            // Convert input to user message
            let user_message = if let Some(s) = input.data.as_str() {
                s.to_string()
            } else {
                serde_json::to_string_pretty(&input.data).unwrap_or_default()
            };
            
            // Build messages with system prompt
            let mut messages = vec![ChatMessage::system(&self.config.system_prompt)];
            messages.push(ChatMessage::user(user_message));
            
            let request = ChatRequest::new(messages)
                .with_temperature(0.7)
                .with_max_tokens(500);
            
            // Call LLM
            match client.chat(request).await {
                Ok(response) => {
                    let output_data = serde_json::json!({
                        "response": response.content,
                        "model": response.model,
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
                Err(e) => {
                    Err(AgentError::ExecutionError(format!("LLM call failed: {}", e)))
                }
            }
        } else {
            // Mock execution fallback
            let output_data = serde_json::json!({
                "agent": self.config.name,
                "processed": input.data,
                "system_prompt": self.config.system_prompt,
                "note": "Mock execution - no LLM client configured"
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
}
