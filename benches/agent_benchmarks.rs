use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use agent_runtime::prelude::*;
use agent_runtime::{Agent, AgentConfig, AgentInput, ToolRegistry, NativeTool, Event, EventType};
use serde_json::json;
use std::sync::Arc;

/// Benchmark basic agent execution with MockLlmClient (no tools)
fn bench_agent_execution_no_tools(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("agent_execution_no_tools", |b| {
        b.to_async(&rt).iter(|| async {
            let mock_client = MockLlmClient::new()
                .with_response("Test response from agent");
            
            let config = AgentConfig::builder("bench_agent")
                .system_prompt("You are a test agent")
                .build();
            
            let agent = Agent::new(config)
                .with_llm_client(Arc::new(mock_client));
            
            let input = AgentInput::from_text("test input");
            
            black_box(agent.execute(&input).await.unwrap())
        });
    });
}

/// Benchmark agent execution with 1 tool call
fn bench_agent_with_single_tool(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("agent_execution_single_tool", |b| {
        b.to_async(&rt).iter(|| async {
            // Setup mock client with tool call response
            let mock_client = MockLlmClient::new()
                .with_tool_call("calculator", json!({"operation": "add", "a": 5, "b": 3}))
                .with_response("The result is 8");
            
            // Setup tool registry
            let mut registry = ToolRegistry::new();
            registry.register(NativeTool::new(
                "calculator",
                "Performs arithmetic",
                json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    }
                }),
                |args| Box::pin(async move {
                    let start = std::time::Instant::now();
                    let a = args["a"].as_f64().unwrap();
                    let b = args["b"].as_f64().unwrap();
                    let result = a + b;
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    Ok(ToolResult::success(json!({"result": result}), duration))
                })
            ));
            
            let config = AgentConfig::builder("bench_agent")
                .system_prompt("You are a calculator agent")
                .tools(Arc::new(registry))
                .build();
            
            let agent = Agent::new(config)
                .with_llm_client(Arc::new(mock_client));
            
            let input = AgentInput::from_text("What is 5 + 3?");
            
            black_box(agent.execute(&input).await.unwrap())
        });
    });
}

/// Benchmark agent execution with multiple tool calls (3 calls)
fn bench_agent_with_multiple_tools(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("agent_execution_multiple_tools", |b| {
        b.to_async(&rt).iter(|| async {
            // Setup mock client with multiple tool calls
            let mock_client = MockLlmClient::new()
                .with_tool_call("calculator", json!({"operation": "add", "a": 5, "b": 3}))
                .with_tool_call("calculator", json!({"operation": "multiply", "a": 8, "b": 2}))
                .with_tool_call("calculator", json!({"operation": "subtract", "a": 16, "b": 4}))
                .with_response("The final result is 12");
            
            // Setup tool registry
            let mut registry = ToolRegistry::new();
            registry.register(NativeTool::new(
                "calculator",
                "Performs arithmetic",
                json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "a": {"type": "number"},
                        "b": {"type": "number"}
                    }
                }),
                |args| Box::pin(async move {
                    let start = std::time::Instant::now();
                    let op = args["operation"].as_str().unwrap();
                    let a = args["a"].as_f64().unwrap();
                    let b = args["b"].as_f64().unwrap();
                    
                    let result = match op {
                        "add" => a + b,
                        "subtract" => a - b,
                        "multiply" => a * b,
                        "divide" => a / b,
                        _ => 0.0,
                    };
                    
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    Ok(ToolResult::success(json!({"result": result}), duration))
                })
            ));
            
            let config = AgentConfig::builder("bench_agent")
                .system_prompt("You are a calculator agent")
                .tools(Arc::new(registry))
                .build();
            
            let agent = Agent::new(config)
                .with_llm_client(Arc::new(mock_client));
            
            let input = AgentInput::from_text("Calculate (5 + 3) * 2 - 4");
            
            black_box(agent.execute(&input).await.unwrap())
        });
    });
}

/// Benchmark tool execution overhead (just the tool, no agent)
fn bench_tool_execution_overhead(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("tool_execution_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            let mut registry = ToolRegistry::new();
            registry.register(NativeTool::new(
                "calculator",
                "Simple math",
                json!({}),
                |args| Box::pin(async move {
                    let start = std::time::Instant::now();
                    let a = args["a"].as_f64().unwrap_or(1.0);
                    let b = args["b"].as_f64().unwrap_or(1.0);
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    Ok(ToolResult::success(json!({"result": a + b}), duration))
                })
            ));
            
            let tool = registry.get("calculator").unwrap();
            let mut args = std::collections::HashMap::new();
            args.insert("a".to_string(), json!(5));
            args.insert("b".to_string(), json!(3));
            
            black_box(tool.execute(args).await.unwrap())
        });
    });
}

/// Benchmark event emission overhead
fn bench_event_emission(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("event_emission", |b| {
        b.to_async(&rt).iter(|| async {
            let (tx, mut rx) = tokio::sync::broadcast::channel(100);
            
            // Emit event
            let event = Event::new(
                0,
                EventType::AgentProcessing,
                "bench_workflow".to_string(),
                json!({"agent": "bench_agent", "step": 0})
            );
            
            let _ = tx.send(black_box(event));
            
            // Receive event
            black_box(rx.recv().await.ok())
        });
    });
}

/// Benchmark varying number of concurrent agents
fn bench_concurrent_agents(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_agents");
    
    for num_agents in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(num_agents), num_agents, |b, &num| {
            b.to_async(&rt).iter(|| async move {
                let mut handles = vec![];
                
                for i in 0..num {
                    let handle = tokio::spawn(async move {
                        let response_text = format!("Response from agent {}", i);
                        let mock_client = MockLlmClient::new()
                            .with_response(&response_text);
                        
                        let config = AgentConfig::builder(&format!("agent_{}", i))
                            .system_prompt("Test agent")
                            .build();
                        
                        let agent = Agent::new(config)
                            .with_llm_client(Arc::new(mock_client));
                        
                        let input = AgentInput::from_text("test");
                        agent.execute(&input).await.unwrap()
                    });
                    handles.push(handle);
                }
                
                black_box(futures::future::join_all(handles).await)
            });
        });
    }
    group.finish();
}

/// Benchmark tool loop detection overhead
fn bench_tool_loop_detection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("tool_loop_detection", |b| {
        b.to_async(&rt).iter(|| async {
            // Setup mock with duplicate tool calls to trigger detection
            let mock_client = MockLlmClient::new()
                .with_tool_call("calculator", json!({"a": 5, "b": 3}))
                .with_tool_call("calculator", json!({"a": 5, "b": 3})) // Duplicate!
                .with_response("Result");
            
            let mut registry = ToolRegistry::new();
            registry.register(NativeTool::new(
                "calculator",
                "Math",
                json!({}),
                |args| Box::pin(async move {
                    let start = std::time::Instant::now();
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    Ok(ToolResult::success(json!({"result": 8}), duration))
                })
            ));
            
            let config = AgentConfig::builder("bench_agent")
                .system_prompt("Test")
                .tools(Arc::new(registry))
                .build();
            
            let agent = Agent::new(config)
                .with_llm_client(Arc::new(mock_client));
            
            let input = AgentInput::from_text("test");
            black_box(agent.execute(&input).await.ok())
        });
    });
}

criterion_group!(
    benches,
    bench_agent_execution_no_tools,
    bench_agent_with_single_tool,
    bench_agent_with_multiple_tools,
    bench_tool_execution_overhead,
    bench_event_emission,
    bench_concurrent_agents,
    bench_tool_loop_detection
);

criterion_main!(benches);
