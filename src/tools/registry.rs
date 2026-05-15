use crate::types::{ToolError, ToolExecutionResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Tool trait that all tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique name for this tool
    fn name(&self) -> &str;

    /// Human-readable description for LLM
    fn description(&self) -> &str;

    /// JSON schema for input parameters
    fn input_schema(&self) -> JsonValue;

    /// Execute the tool with given parameters
    async fn execute(&self, params: HashMap<String, JsonValue>) -> ToolExecutionResult;
}

/// Registry for managing tools
///
/// The registry stores all available tools and provides methods to
/// list, query, and execute them.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    ///
    /// # Arguments
    /// * `tool` - The tool to register (must implement `Tool` trait)
    ///
    /// # Returns
    /// * `&mut Self` - For method chaining
    pub fn register(&mut self, tool: impl Tool + 'static) -> &mut Self {
        let name = tool.name().to_string();
        self.tools.insert(name, Arc::new(tool));
        self
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// List all tool names
    pub fn list_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// List all tools with their schemas (for LLM function calling)
    pub fn list_tools(&self) -> Vec<JsonValue> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.input_schema(),
                    }
                })
            })
            .collect()
    }

    /// Call a tool by name with the given parameters
    pub async fn call_tool(
        &self,
        name: &str,
        params: HashMap<String, JsonValue>,
    ) -> ToolExecutionResult {
        match self.tools.get(name) {
            Some(tool) => tool.execute(params).await,
            None => Err(ToolError::InvalidParameters(format!(
                "Tool not found: {}",
                name
            ))),
        }
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tool_count", &self.tools.len())
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
}
