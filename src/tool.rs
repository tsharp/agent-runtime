use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use crate::types::{ToolExecutionResult, ToolResult, ToolError};

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
        
        let message = params.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'message' parameter".into()))?;
        
        let output = serde_json::json!({
            "echoed": message
        });
        
        Ok(ToolResult {
            output,
            duration_ms: start.elapsed().as_millis() as u64,
        })
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
        
        let operation = params.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'operation'".into()))?;
        
        let a = params.get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'a'".into()))?;
        
        let b = params.get("b")
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
            _ => return Err(ToolError::InvalidParameters(format!("unknown operation: {}", operation))),
        };
        
        Ok(ToolResult {
            output: serde_json::json!({ "result": result }),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
