use agent_runtime::llm::LlamaClient;
use agent_runtime::tool::{NativeTool, ToolRegistry};
use agent_runtime::types::{AgentInputMetadata, ToolError, ToolResult};
use agent_runtime::{Agent, AgentConfig, AgentInput, EventType, FileLogger, Runtime};
use serde_json::json;
use std::fs;
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Agent with Tools Demo (llama.cpp) ===\n");
    
    // Create output directory
    fs::create_dir_all("output").expect("Failed to create output directory");
    
    // Create file logger
    let logger = FileLogger::new("output/agent_with_tools_demo.log")
        .expect("Failed to create log file");
    logger.log("=== Agent with Tools Demo Started ===");

    // Create tool registry
    let mut registry = ToolRegistry::new();

    // Register calculator tools
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
            let a = params.get("a").and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidParameters("'a' must be a number".into()))?;
            let b = params.get("b").and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidParameters("'b' must be a number".into()))?;
            Ok(ToolResult {
                output: json!({ "result": a + b }),
                duration_ms: start.elapsed().as_millis() as u64,
            })
        },
    ));

    registry.register(NativeTool::new(
        "multiply",
        "Multiply two numbers",
        json!({
            "type": "object",
            "properties": {
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["a", "b"]
        }),
        |params| async move {
            let start = std::time::Instant::now();
            let a = params.get("a").and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidParameters("'a' must be a number".into()))?;
            let b = params.get("b").and_then(|v| v.as_f64())
                .ok_or_else(|| ToolError::InvalidParameters("'b' must be a number".into()))?;
            Ok(ToolResult {
                output: json!({ "result": a * b }),
                duration_ms: start.elapsed().as_millis() as u64,
            })
        },
    ));

    registry.register(NativeTool::new(
        "get_weather",
        "Get the current weather for a city",
        json!({
            "type": "object",
            "properties": {
                "city": { "type": "string", "description": "City name" }
            },
            "required": ["city"]
        }),
        |params| async move {
            let start = std::time::Instant::now();
            let city = params.get("city").and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("'city' must be a string".into()))?;
            
            // Mock weather data
            let weather = match city.to_lowercase().as_str() {
                "london" => "Rainy, 15°C",
                "tokyo" => "Sunny, 22°C",
                "new york" => "Cloudy, 18°C",
                _ => "Unknown, check weather.com",
            };
            
            Ok(ToolResult {
                output: json!({ "weather": weather, "city": city }),
                duration_ms: start.elapsed().as_millis() as u64,
            })
        },
    ));

    println!("📋 Registered Tools:");
    for name in registry.list_names() {
        if let Some(tool) = registry.get(&name) {
            println!("  • {} - {}", tool.name(), tool.description());
        }
    }
    println!();

    // Create llama.cpp client (LM Studio)
    let base_url = "http://localhost:1234/v1";
    let model = "qwen/qwen3-30b-a3b-2507";
    
    println!("🦙 Connecting to llama.cpp at {}", base_url);
    println!("   Model: {}\n", model);
    logger.log(&format!("Connecting to llama.cpp at {} (model: {})", base_url, model));
    
    let llm_client = Arc::new(LlamaClient::new(base_url, model));
    
    // Create runtime for event streaming
    let runtime = Runtime::new();
    
    // Subscribe to events for logging
    let mut event_receiver = runtime.event_stream().subscribe();
    let logger_for_events = logger.clone();
    let _event_task = task::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            // Log all events to file
            logger_for_events.log_level(
                &format!("{:?}", event.event_type),
                &serde_json::to_string(&event.data).unwrap_or_default()
            );
            
            // Print tool call events to console
            match event.event_type {
                EventType::ToolCallStarted => {
                    if let Some(tool) = event.data.get("tool_name").and_then(|v| v.as_str()) {
                        println!("   🔧 Calling tool: {}", tool);
                    }
                }
                EventType::ToolCallCompleted => {
                    if let Some(tool) = event.data.get("tool_name").and_then(|v| v.as_str()) {
                        if let Some(duration) = event.data.get("duration_ms") {
                            println!("   ✓ Tool {} completed in {}ms", tool, duration);
                        }
                    }
                }
                _ => {}
            }
        }
    });

    // Create agent with tools
    let agent = Agent::new(
        AgentConfig::builder("math_assistant")
            .system_prompt(
                "You are a helpful assistant with access to calculator and weather tools. \
                 Use the tools when needed to answer user questions accurately.",
            )
            .tools(Arc::new(registry))
            .max_tool_iterations(5)
            .build(),
    )
    .with_llm_client(llm_client);

    // Test 1: Simple calculation
    {
        let test_num = 1;
        let desc = "Ask agent to calculate something";
        let question = "What is 15 + 27?";
        
        println!("🧮 Test {}: {}", test_num, desc);
        println!("   Question: {}", question);
        logger.log(&format!("Test {}: {} - Question: {}", test_num, desc, question));
        
        let input = AgentInput {
            data: json!(question),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
        };
        
        match agent.execute_with_events(input, Some(runtime.event_stream())).await {
            Ok(output) => {
                if let Some(response) = output.data.get("response").and_then(|v| v.as_str()) {
                    println!("   ✅ Response: {}", response);
                    logger.log(&format!("Test {} result: {}", test_num, response));
                } else {
                    println!("   ✅ Response: {}", output.data);
                    logger.log(&format!("Test {} result: {}", test_num, output.data));
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
                logger.log(&format!("Test {} error: {}", test_num, e));
            }
        }
        println!();
    }

    // Test 2: Multi-step calculation
    {
        let test_num = 2;
        let desc = "Multi-step calculation";
        let question = "What is (5 + 3) * 4?";
        
        println!("🧮 Test {}: {}", test_num, desc);
        println!("   Question: {}", question);
        logger.log(&format!("Test {}: {} - Question: {}", test_num, desc, question));
        
        let input = AgentInput {
            data: json!(question),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
        };
        
        match agent.execute_with_events(input, Some(runtime.event_stream())).await {
            Ok(output) => {
                if let Some(response) = output.data.get("response").and_then(|v| v.as_str()) {
                    println!("   ✅ Response: {}", response);
                    logger.log(&format!("Test {} result: {}", test_num, response));
                } else {
                    println!("   ✅ Response: {}", output.data);
                    logger.log(&format!("Test {} result: {}", test_num, output.data));
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
                logger.log(&format!("Test {} error: {}", test_num, e));
            }
        }
        println!();
    }

    // Test 3: Weather query
    {
        let test_num = 3;
        let desc = "Weather query";
        let question = "What's the weather in Tokyo?";
        
        println!("🌤️  Test {}: {}", test_num, desc);
        println!("   Question: {}", question);
        logger.log(&format!("Test {}: {} - Question: {}", test_num, desc, question));
        
        let input = AgentInput {
            data: json!(question),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
        };
        
        match agent.execute_with_events(input, Some(runtime.event_stream())).await {
            Ok(output) => {
                if let Some(response) = output.data.get("response").and_then(|v| v.as_str()) {
                    println!("   ✅ Response: {}", response);
                    logger.log(&format!("Test {} result: {}", test_num, response));
                } else {
                    println!("   ✅ Response: {}", output.data);
                    logger.log(&format!("Test {} result: {}", test_num, output.data));
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
                logger.log(&format!("Test {} error: {}", test_num, e));
            }
        }
        println!();
    }

    // Test 4: Mixed tools
    {
        let test_num = 4;
        let desc = "Mixed tools";
        let question = "If it's 22°C in Tokyo and 15°C in London, what's the temperature difference? Use the weather tools to get the actual temperatures.";
        
        println!("🔀 Test {}: {}", test_num, desc);
        println!("   Question: {}", question);
        logger.log(&format!("Test {}: {} - Question: {}", test_num, desc, question));
        
        let input = AgentInput {
            data: json!(question),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
        };
        
        match agent.execute_with_events(input, Some(runtime.event_stream())).await {
            Ok(output) => {
                if let Some(response) = output.data.get("response").and_then(|v| v.as_str()) {
                    println!("   ✅ Response: {}", response);
                    logger.log(&format!("Test {} result: {}", test_num, response));
                } else {
                    println!("   ✅ Response: {}", output.data);
                    logger.log(&format!("Test {} result: {}", test_num, output.data));
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
                logger.log(&format!("Test {} error: {}", test_num, e));
            }
        }
        println!();
    }
    
    // Save summary
    println!("💾 Logs and results saved to output/");
    println!("   - agent_with_tools_demo.log (debug log with all events)");
    logger.log("=== Agent with Tools Demo Completed ===");

    Ok(())
}
