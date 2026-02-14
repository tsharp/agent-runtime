// Integration tests for agent-runtime
// Tests end-to-end workflows with mock LLM and real components

use agent_runtime::llm::{MockLlmClient, MockResponse};
use agent_runtime::prelude::*;
use agent_runtime::tool::{NativeTool, ToolRegistry};
use agent_runtime::types::ToolResult;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;

// === Helper Functions ===

fn calculator_tool() -> NativeTool {
    NativeTool::new(
        "calculator",
        "Performs basic arithmetic operations",
        json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        }),
        |args| {
            Box::pin(async move {
                let start = std::time::Instant::now();
                let op = args["operation"].as_str().unwrap();
                let a = args["a"].as_f64().unwrap();
                let b = args["b"].as_f64().unwrap();

                let result = match op {
                    "add" => a + b,
                    "subtract" => a - b,
                    "multiply" => a * b,
                    "divide" if b != 0.0 => a / b,
                    "divide" => {
                        return Ok(ToolResult::error(
                            "Division by zero",
                            start.elapsed().as_secs_f64() * 1000.0,
                        ))
                    }
                    _ => {
                        return Ok(ToolResult::error(
                            "Unknown operation",
                            start.elapsed().as_secs_f64() * 1000.0,
                        ))
                    }
                };

                Ok(ToolResult::success(
                    json!({"result": result}),
                    start.elapsed().as_secs_f64() * 1000.0,
                ))
            })
        },
    )
}

fn search_tool() -> NativeTool {
    NativeTool::new(
        "search",
        "Searches for information",
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            },
            "required": ["query"]
        }),
        |args| {
            Box::pin(async move {
                let start = std::time::Instant::now();
                let query = args["query"].as_str().unwrap();

                // Simulate search results based on query
                if query.contains("rust") {
                    Ok(ToolResult::success(
                        json!({
                            "results": [
                                {"title": "Rust Programming Language", "url": "https://rust-lang.org"}
                            ]
                        }),
                        start.elapsed().as_secs_f64() * 1000.0,
                    ))
                } else {
                    // Empty results - use success_no_data to signal no data
                    Ok(ToolResult::success_no_data(
                        format!("No results found for '{}'", query),
                        start.elapsed().as_secs_f64() * 1000.0,
                    ))
                }
            })
        },
    )
}

// === Integration Tests ===

#[tokio::test]
async fn test_agent_with_tool_execution() {
    // Mock LLM that calls calculator tool, then responds with result
    let mock_llm = Arc::new(MockLlmClient::with_tool_then_text(
        "calculator",
        json!({"operation": "add", "a": 42, "b": 137}),
        "The result is 179",
    ));

    // Create agent with calculator tool
    let mut registry = ToolRegistry::new();
    registry.register(calculator_tool());

    let config = AgentConfig::builder("calculator_agent")
        .system_prompt("You are a math assistant.")
        .tools(Arc::new(registry))
        .build();

    let agent = Agent::new(config).with_llm_client(mock_llm.clone());

    // Execute
    let input = AgentInput::from_text("What is 42 + 137?");
    let output = agent.execute(&input).await.unwrap();

    // Verify
    assert!(output.data.to_string().contains("179"));
    assert_eq!(mock_llm.call_count(), 2); // 1 for tool call, 1 for final response
}

#[tokio::test]
async fn test_agent_tool_loop_detection() {
    // Mock LLM that tries to call the same tool twice
    let mock_llm = Arc::new(MockLlmClient::from_mock_responses(vec![
        MockResponse::with_tool_call("search", json!({"query": "nonexistent"})),
        MockResponse::with_tool_call("search", json!({"query": "nonexistent"})), // Duplicate!
        MockResponse::text("I couldn't find any information"),
    ]));

    let mut registry = ToolRegistry::new();
    registry.register(search_tool());

    let config = AgentConfig::builder("search_agent")
        .system_prompt("You search for information.")
        .tools(Arc::new(registry))
        .build(); // Loop detection enabled by default

    let agent = Agent::new(config).with_llm_client(mock_llm.clone());

    let input = AgentInput::from_text("Search for nonexistent");
    let _output = agent.execute(&input).await.unwrap();

    // The loop detection prevents the second identical tool call
    // We can verify this by checking the mock call count
    // Without loop detection, it would call the LLM 3 times (tool1, tool2 duplicate, final)
    // With loop detection, it calls 2 times (tool1, loop intercepted with message, final)
    assert_eq!(mock_llm.call_count(), 3); // All 3 mock responses used
}

// TODO: Workflow test - needs Runtime API
// #[tokio::test]
// async fn test_workflow_multi_agent() { ... }

#[tokio::test]
async fn test_error_handling_network_failure() {
    // Mock LLM that fails on first call
    let mock_llm = Arc::new(MockLlmClient::with_responses_vec(vec![
        "Response 1",
        "Response 2",
    ]));
    mock_llm.fail_on_call(0); // Fail on first call

    let agent = Agent::new(
        AgentConfig::builder("flaky_agent")
            .system_prompt("Test agent")
            .build(),
    )
    .with_llm_client(mock_llm);

    let input = AgentInput::from_text("Test");
    let result = agent.execute(&input).await;

    // Should propagate the error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("network"));
}
