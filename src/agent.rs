use crate::event::{EventStream, EventType};
use crate::llm::types::ToolCall;
use crate::llm::{ChatClient, ChatMessage, ChatRequest};
use crate::tool::ToolRegistry;
use crate::tool_loop_detection::{ToolCallTracker, ToolLoopDetectionConfig};
use crate::types::{AgentError, AgentInput, AgentOutput, AgentOutputMetadata, AgentResult, ToolStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
#[path = "agent_test.rs"]
mod agent_test;

/// Agent configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub system_prompt: String,

    #[serde(skip)]
    pub tools: Option<Arc<ToolRegistry>>,
    
    pub max_tool_iterations: usize,
    
    /// Tool loop detection configuration
    #[serde(skip)]
    pub tool_loop_detection: Option<ToolLoopDetectionConfig>,
}

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentConfig")
            .field("name", &self.name)
            .field("system_prompt", &self.system_prompt)
            .field("tools", &self.tools.as_ref().map(|t| format!("{} tools", t.len())))
            .field("max_tool_iterations", &self.max_tool_iterations)
            .field("tool_loop_detection", &self.tool_loop_detection.as_ref().map(|c| c.enabled))
            .finish()
    }
}

impl AgentConfig {
    pub fn builder(name: impl Into<String>) -> AgentConfigBuilder {
        AgentConfigBuilder {
            name: name.into(),
            system_prompt: String::new(),
            tools: None,
            max_tool_iterations: 10,
            tool_loop_detection: Some(ToolLoopDetectionConfig::default()),
        }
    }
}

/// Builder for AgentConfig
pub struct AgentConfigBuilder {
    name: String,
    system_prompt: String,
    tools: Option<Arc<ToolRegistry>>,
    max_tool_iterations: usize,
    tool_loop_detection: Option<ToolLoopDetectionConfig>,
}

impl AgentConfigBuilder {
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn tools(mut self, tools: Arc<ToolRegistry>) -> Self {
        self.tools = Some(tools);
        self
    }
    
    pub fn max_tool_iterations(mut self, max: usize) -> Self {
        self.max_tool_iterations = max;
        self
    }
    
    pub fn tool_loop_detection(mut self, config: ToolLoopDetectionConfig) -> Self {
        self.tool_loop_detection = Some(config);
        self
    }
    
    pub fn disable_tool_loop_detection(mut self) -> Self {
        self.tool_loop_detection = None;
        self
    }

    pub fn build(self) -> AgentConfig {
        AgentConfig {
            name: self.name,
            system_prompt: self.system_prompt,
            tools: self.tools,
            max_tool_iterations: self.max_tool_iterations,
            tool_loop_detection: self.tool_loop_detection,
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
    pub async fn execute(&self, input: &AgentInput) -> AgentResult {
        self.execute_with_events(input.clone(), None).await
    }

    /// Execute the agent with event stream for observability
    pub async fn execute_with_events(
        &self,
        input: AgentInput,
        event_stream: Option<&EventStream>,
    ) -> AgentResult {
        let start = std::time::Instant::now();

        // Emit agent processing event
        if let Some(stream) = event_stream {
            stream.append(
                EventType::AgentProcessing,
                input
                    .metadata
                    .previous_agent
                    .clone()
                    .unwrap_or_else(|| "workflow".to_string()),
                serde_json::json!({
                    "agent": self.config.name,
                    "input": input.data,
                }),
            );
        }

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
            messages.push(ChatMessage::user(&user_message));

            let mut request = ChatRequest::new(messages.clone())
                .with_temperature(0.7)
                .with_max_tokens(8192);
            
            // Get tool schemas if available
            let tool_schemas = self.config.tools.as_ref()
                .map(|registry| registry.list_tools())
                .filter(|tools| !tools.is_empty());
            
            // Tool calling loop
            let mut iteration = 0;
            let mut total_tool_calls = 0;
            
            // Initialize tool call tracker for loop detection
            let mut tool_tracker = if self.config.tool_loop_detection.is_some() {
                Some(ToolCallTracker::new())
            } else {
                None
            };
            
            loop {
                iteration += 1;
                
                // Check iteration limit
                if iteration > self.config.max_tool_iterations {
                    return Err(AgentError::ExecutionError(format!(
                        "Maximum tool iterations ({}) exceeded",
                        self.config.max_tool_iterations
                    )));
                }
                
                // Add tools to request if available
                if let Some(ref schemas) = tool_schemas {
                    request = request.with_tools(schemas.clone());
                }

                // Emit LLM request started event
                if let Some(stream) = event_stream {
                    stream.append(
                        EventType::AgentLlmRequestStarted,
                        input
                            .metadata
                            .previous_agent
                            .clone()
                            .unwrap_or_else(|| "workflow".to_string()),
                        serde_json::json!({
                            "agent": self.config.name,
                            "iteration": iteration,
                        }),
                    );
                }

                // Call LLM with streaming + full response (for tool calls)
                let event_stream_for_streaming = event_stream.cloned();
                let agent_name = self.config.name.clone();
                let previous_agent = input
                    .metadata
                    .previous_agent
                    .clone()
                    .unwrap_or_else(|| "workflow".to_string());
                
                // Create channel for streaming chunks
                let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel(100);
                
                // Spawn task to receive chunks and emit events
                let chunk_event_task = tokio::spawn(async move {
                    while let Some(chunk) = chunk_rx.recv().await {
                        if let Some(stream) = &event_stream_for_streaming {
                            stream.append(
                                EventType::AgentLlmStreamChunk,
                                previous_agent.clone(),
                                serde_json::json!({
                                    "agent": &agent_name,
                                    "chunk": chunk,
                                }),
                            );
                        }
                    }
                });
                
                match client.chat_stream(request.clone(), chunk_tx).await {
                    Ok(response) => {
                        
                        // Emit LLM request completed event
                        if let Some(stream) = event_stream {
                            stream.append(
                                EventType::AgentLlmRequestCompleted,
                                input
                                    .metadata
                                    .previous_agent
                                    .clone()
                                    .unwrap_or_else(|| "workflow".to_string()),
                                serde_json::json!({
                                    "agent": self.config.name,
                                }),
                            );
                        }
                        
                        // Check if we have tool calls (and they're not empty)
                        if let Some(tool_calls) = response.tool_calls.clone() {
                            
                            if tool_calls.is_empty() {
                                // Empty tool calls array - treat as final response
                            } else {
                                
                                total_tool_calls += tool_calls.len();
                                
                                // Add assistant message with tool calls to conversation
                                let assistant_msg = ChatMessage::assistant_with_tool_calls(
                                    response.content.clone(),
                                    tool_calls.clone()
                                );
                                request.messages.push(assistant_msg);
                                
                                // Execute each tool call
                                for tool_call in tool_calls {
                                    // Check for duplicate tool call (loop detection)
                                    if let (Some(tracker), Some(loop_config)) = (&tool_tracker, &self.config.tool_loop_detection) {
                                        if loop_config.enabled {
                                            // Parse tool arguments from JSON string
                                            let args_value: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                                                .unwrap_or(serde_json::json!({}));
                                            
                                            // Convert to HashMap for comparison
                                            let args_map: HashMap<String, serde_json::Value> = args_value
                                                .as_object()
                                                .map(|obj| {
                                                    obj.iter()
                                                        .map(|(k, v)| (k.clone(), v.clone()))
                                                        .collect()
                                                })
                                                .unwrap_or_default();
                                            
                                            if let Some(previous_result) = tracker.check_for_loop(&tool_call.function.name, &args_map) {
                                                // Loop detected! Inject message instead of calling tool
                                                let loop_message = loop_config.get_message(&tool_call.function.name, &previous_result);
                                                
                                                // Emit loop detected event
                                                if let Some(stream) = event_stream {
                                                    stream.append(
                                                        EventType::AgentToolLoopDetected,
                                                        input.metadata.previous_agent.clone().unwrap_or_else(|| "workflow".to_string()),
                                                        serde_json::json!({
                                                            "agent": self.config.name,
                                                            "tool": tool_call.function.name,
                                                            "message": loop_message,
                                                        }),
                                                    );
                                                }
                                                
                                                // Add system message explaining the loop
                                                let tool_msg = ChatMessage::tool_result(
                                                    &tool_call.id,
                                                    &loop_message
                                                );
                                                request.messages.push(tool_msg);
                                                
                                                // Skip actual tool execution
                                                continue;
                                            }
                                        }
                                    }
                                    
                                    // No loop detected - execute the tool normally
                                    let tool_result = self.execute_tool_call(
                                        &tool_call,
                                        &input.metadata.previous_agent.clone().unwrap_or_else(|| "workflow".to_string()),
                                        event_stream
                                    ).await;
                                    
                                    // Record this call in the tracker
                                    if let Some(tracker) = &mut tool_tracker {
                                        // Parse tool arguments from JSON string
                                        let args_value: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                                            .unwrap_or(serde_json::json!({}));
                                        
                                        // Convert to HashMap
                                        let args_map: HashMap<String, serde_json::Value> = args_value
                                            .as_object()
                                            .map(|obj| {
                                                obj.iter()
                                                    .map(|(k, v)| (k.clone(), v.clone()))
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        
                                        let result_json = serde_json::to_value(&tool_result).unwrap_or(serde_json::json!({}));
                                        tracker.record_call(&tool_call.function.name, &args_map, &result_json);
                                    }
                                
                                    // Add tool result to conversation
                                    let tool_msg = ChatMessage::tool_result(
                                        &tool_call.id,
                                        &tool_result
                                    );
                                    request.messages.push(tool_msg);
                                }
                            
                            // Continue loop to get next response
                            continue;
                            }
                        }
                        
                        // No tool calls (or empty array), we have the final response
                        let response_text = response.content.trim();
                        let token_count = response.usage
                            .map(|u| u.total_tokens)
                            .unwrap_or_else(|| (response_text.len() as f32 / 4.0).ceil() as u32);

                        let output_data = serde_json::json!({
                            "response": response_text,
                            "content_type": "text/plain",
                            "token_count": token_count,
                        });

                        // Emit agent completed event
                        if let Some(stream) = event_stream {
                            stream.append(
                                EventType::AgentCompleted,
                                input
                                    .metadata
                                    .previous_agent
                                    .clone()
                                    .unwrap_or_else(|| "workflow".to_string()),
                                serde_json::json!({
                                    "agent": self.config.name,
                                    "execution_time_ms": start.elapsed().as_millis() as u64,
                                }),
                            );
                        }

                        return Ok(AgentOutput {
                            data: output_data,
                            metadata: AgentOutputMetadata {
                                agent_name: self.config.name.clone(),
                                execution_time_ms: start.elapsed().as_millis() as u64,
                                tool_calls_count: total_tool_calls,
                            },
                        });
                    }
                    Err(e) => {
                        // Emit LLM request failed event
                        if let Some(stream) = event_stream {
                            stream.append(
                                EventType::AgentLlmRequestFailed,
                                input
                                    .metadata
                                    .previous_agent
                                    .clone()
                                    .unwrap_or_else(|| "workflow".to_string()),
                                serde_json::json!({
                                    "agent": self.config.name,
                                    "error": e.to_string(),
                                }),
                            );
                        }

                        // Emit agent failed event
                        if let Some(stream) = event_stream {
                            stream.append(
                                EventType::AgentFailed,
                                input
                                    .metadata
                                    .previous_agent
                                    .clone()
                                    .unwrap_or_else(|| "workflow".to_string()),
                                serde_json::json!({
                                    "agent": self.config.name,
                                    "error": e.to_string(),
                                }),
                            );
                        }

                        return Err(AgentError::ExecutionError(format!(
                            "LLM call failed: {}",
                            e
                        )));
                    }
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

            if let Some(stream) = event_stream {
                stream.append(
                    EventType::AgentCompleted,
                    input
                        .metadata
                        .previous_agent
                        .clone()
                        .unwrap_or_else(|| "workflow".to_string()),
                    serde_json::json!({
                        "agent": self.config.name,
                        "execution_time_ms": start.elapsed().as_millis() as u64,
                        "mock": true,
                    }),
                );
            }

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
    
    /// Execute a single tool call
    async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        previous_agent: &str,
        event_stream: Option<&EventStream>,
    ) -> String {
        let tool_name = &tool_call.function.name;
        
        // Emit tool call started event
        if let Some(stream) = event_stream {
            stream.append(
                EventType::ToolCallStarted,
                previous_agent.to_string(),
                serde_json::json!({
                    "agent": self.config.name,
                    "tool": tool_name,
                    "tool_call_id": tool_call.id,
                    "arguments": tool_call.function.arguments,
                }),
            );
        }
        
        // Get the tool registry
        let registry = match &self.config.tools {
            Some(reg) => reg,
            None => {
                let error_msg = "No tool registry configured".to_string();
                if let Some(stream) = event_stream {
                    stream.append(
                        EventType::ToolCallFailed,
                        previous_agent.to_string(),
                        serde_json::json!({
                            "agent": self.config.name,
                            "tool": tool_name,
                            "tool_call_id": tool_call.id,
                            "arguments": tool_call.function.arguments,
                            "error": error_msg,
                            "duration_ms": 0,
                        }),
                    );
                }
                return format!("Error: {}", error_msg);
            }
        };
        
        // Parse arguments from JSON string
        let params: HashMap<String, serde_json::Value> = match serde_json::from_str(&tool_call.function.arguments) {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("Failed to parse tool arguments: {}", e);
                if let Some(stream) = event_stream {
                    stream.append(
                        EventType::ToolCallFailed,
                        previous_agent.to_string(),
                        serde_json::json!({
                            "agent": self.config.name,
                            "tool": tool_name,
                            "tool_call_id": tool_call.id,
                            "arguments": tool_call.function.arguments,
                            "error": error_msg,
                            "duration_ms": 0,
                        }),
                    );
                }
                return format!("Error: {}", error_msg);
            }
        };
        
        // Execute the tool
        let start_time = std::time::Instant::now();
        match registry.call_tool(tool_name, params.clone()).await {
            Ok(result) => {
                // Emit tool call completed event
                if let Some(stream) = event_stream {
                    stream.append(
                        EventType::ToolCallCompleted,
                        previous_agent.to_string(),
                        serde_json::json!({
                            "agent": self.config.name,
                            "tool": tool_name,
                            "tool_call_id": tool_call.id,
                            "arguments": params,
                            "result": result.output,
                            "duration_ms": (result.duration_ms * 1000.0).round() / 1000.0,
                        }),
                    );
                }
                
                // Convert result to string for LLM
                serde_json::to_string(&result.output).unwrap_or_else(|_| result.output.to_string())
            }
            Err(e) => {
                let error_msg = format!("Tool execution failed: {}", e);
                if let Some(stream) = event_stream {
                    stream.append(
                        EventType::ToolCallFailed,
                        previous_agent.to_string(),
                        serde_json::json!({
                            "agent": self.config.name,
                            "tool": tool_name,
                            "tool_call_id": tool_call.id,
                            "arguments": params,
                            "error": error_msg,
                            "duration_ms": start_time.elapsed().as_secs_f64() * 1000.0,
                        }),
                    );
                }
                format!("Error: {}", error_msg)
            }
        }
    }
}
