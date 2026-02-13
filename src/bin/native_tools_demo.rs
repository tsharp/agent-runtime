use agent_runtime::tool::{NativeTool, ToolRegistry};
use agent_runtime::types::{ToolError, ToolResult};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Native Tools Demo ===\n");

    // Create tool registry
    let mut registry = ToolRegistry::new();

    // Register calculator tool
    registry.register(NativeTool::new(
        "add",
        "Add two numbers together",
        json!({
            "type": "object",
            "properties": {
                "a": { "type": "number", "description": "First number" },
                "b": { "type": "number", "description": "Second number" }
            },
            "required": ["a", "b"]
        }),
        |params| async move {
            let start = std::time::Instant::now();

            let a = params
                .get("a")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidParameters("'a' must be a number".into()))?;

            let b = params
                .get("b")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidParameters("'b' must be a number".into()))?;

            let result = a + b;

            Ok(ToolResult {
                output: json!({ "result": result }),
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            })
        },
    ));

    // Register string processing tool
    registry.register(NativeTool::new(
        "uppercase",
        "Convert text to uppercase",
        json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Text to convert" }
            },
            "required": ["text"]
        }),
        |params| async move {
            let start = std::time::Instant::now();

            let text = params
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("'text' must be a string".into()))?;

            let result = text.to_uppercase();

            Ok(ToolResult {
                output: json!({ "result": result }),
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            })
        },
    ));

    // List available tools
    println!("📋 Registered Tools:");
    for name in registry.list_names() {
        if let Some(tool) = registry.get(&name) {
            println!("  • {} - {}", tool.name(), tool.description());
        }
    }
    println!();

    // Test adding numbers
    println!("🧮 Testing 'add' tool:");
    let mut params = HashMap::new();
    params.insert("a".to_string(), json!(5.0));
    params.insert("b".to_string(), json!(3.0));

    match registry.call_tool("add", params).await {
        Ok(result) => {
            println!(
                "   Result: {} (took {}ms)",
                result.output, result.duration_ms
            );
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Test string uppercasing
    println!("🔤 Testing 'uppercase' tool:");
    let mut params = HashMap::new();
    params.insert("text".to_string(), json!("hello world"));

    match registry.call_tool("uppercase", params).await {
        Ok(result) => {
            println!(
                "   Result: {} (took {}ms)",
                result.output, result.duration_ms
            );
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Test tool not found
    println!("❌ Testing non-existent tool:");
    match registry.call_tool("nonexistent", HashMap::new()).await {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   Error (expected): {}", e),
    }
    println!();

    // Show tool schemas for LLM
    println!("📝 Tool Schemas (for LLM function calling):");
    for schema in registry.list_tools() {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    }

    Ok(())
}
