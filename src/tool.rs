use crate::types::{ToolError, ToolExecutionResult, ToolResult};
use async_trait::async_trait;
use futures::future::BoxFuture;
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

type ToolExecutor =
    Arc<dyn Fn(HashMap<String, JsonValue>) -> BoxFuture<'static, ToolExecutionResult> + Send + Sync>;

/// A native (in-memory) tool implemented as a Rust async function
///
/// Native tools execute directly in the runtime process with no IPC overhead.
/// They are defined as async closures that accept parameters and return results.
pub struct NativeTool {
    name: String,
    description: String,
    input_schema: JsonValue,
    executor: ToolExecutor,
}

impl NativeTool {
    /// Create a new native tool
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the tool
    /// * `description` - Human-readable description
    /// * `input_schema` - JSON Schema describing input parameters
    /// * `executor` - Async function that executes the tool
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: JsonValue,
        executor: F,
    ) -> Self
    where
        F: Fn(HashMap<String, JsonValue>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolExecutionResult> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            executor: Arc::new(move |params| Box::pin(executor(params))),
        }
    }
}

#[async_trait]
impl Tool for NativeTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    async fn execute(&self, params: HashMap<String, JsonValue>) -> ToolExecutionResult {
        (self.executor)(params).await
    }
}

impl std::fmt::Debug for NativeTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .finish()
    }
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

/// Example: Echo tool that returns its input
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes back the input message"
    }

    fn input_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, params: HashMap<String, JsonValue>) -> ToolExecutionResult {
        let start = std::time::Instant::now();

        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'message' parameter".into()))?;

        let output = serde_json::json!({
            "echoed": message
        });

        Ok(ToolResult::success(
            output,
            start.elapsed().as_secs_f64() * 1000.0,
        ))
    }
}

/// Example: Calculator tool for simple math
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Performs basic arithmetic operations (add, subtract, multiply, divide)"
    }

    fn input_schema(&self) -> JsonValue {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["operation", "a", "b"]
        })
    }

    async fn execute(&self, params: HashMap<String, JsonValue>) -> ToolExecutionResult {
        let start = std::time::Instant::now();

        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'operation'".into()))?;

        let a = params
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'a'".into()))?;

        let b = params
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'b'".into()))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(ToolError::ExecutionFailed("division by zero".into()));
                }
                a / b
            }
            _ => {
                return Err(ToolError::InvalidParameters(format!(
                    "unknown operation: {}",
                    operation
                )))
            }
        };

        Ok(ToolResult::success(
            serde_json::json!({ "result": result }),
            start.elapsed().as_secs_f64() * 1000.0,
        ))
    }
}
