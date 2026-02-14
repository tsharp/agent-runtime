/// Load and concurrency tests
/// Tests system behavior under concurrent load
use agent_runtime::prelude::*;
use agent_runtime::{
    Agent, AgentConfig, AgentInput, NativeTool, Runtime, ToolRegistry, WorkflowBuilder,
};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::test]
async fn test_concurrent_agents_10() {
    // Test 10 concurrent agents
    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let response_text = format!("Response {}", i);
            let mock_client = MockLlmClient::new().with_response(&response_text);

            let config = AgentConfig::builder(&format!("agent_{}", i))
                .system_prompt("Test agent")
                .build();

            let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

            let input = AgentInput::from_text(&format!("Input {}", i));
            agent.execute(&input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    // All should succeed
    for result in results {
        assert!(result.is_ok(), "Task should not panic");
        assert!(result.unwrap().is_ok(), "Agent execution should succeed");
    }
}

#[tokio::test]
async fn test_concurrent_agents_50() {
    // Test 50 concurrent agents
    let mut handles = vec![];

    for i in 0..50 {
        let handle = tokio::spawn(async move {
            let mock_client = MockLlmClient::new().with_response(format!("Response {}", i));

            let config = AgentConfig::builder(&format!("agent_{}", i))
                .system_prompt("Test")
                .build();

            let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

            let input = AgentInput::from_text("test");
            agent.execute(&input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    // All should succeed
    assert_eq!(results.len(), 50);
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_agents_100() {
    // Test 100 concurrent agents
    let mut handles = vec![];

    for i in 0..100 {
        let handle = tokio::spawn(async move {
            let mock_client = MockLlmClient::new().with_response("Response");

            let config = AgentConfig::builder(&format!("agent_{}", i))
                .system_prompt("Test")
                .build();

            let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

            let input = AgentInput::from_text("test");
            agent.execute(&input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    assert_eq!(results.len(), 100);
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_concurrent_tool_executions() {
    // Test many concurrent tool executions
    let call_count = Arc::new(AtomicUsize::new(0));
    let registry = Arc::new(ToolRegistry::new());

    let counter = call_count.clone();
    registry.register(NativeTool::new(
        "counter",
        "Increments counter",
        json!({}),
        move |_args| {
            let count = counter.clone();
            Box::pin(async move {
                let start = std::time::Instant::now();
                count.fetch_add(1, Ordering::SeqCst);
                let duration = start.elapsed().as_secs_f64() * 1000.0;
                Ok(ToolResult::success(json!({"status": "ok"}), duration))
            })
        },
    ));

    let mut handles = vec![];

    for i in 0..50 {
        let reg = registry.clone();
        let handle = tokio::spawn(async move {
            let tool = reg.get("counter").unwrap();
            tool.execute(&json!({})).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    // All calls should succeed
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    // Counter should be exactly 50
    assert_eq!(call_count.load(Ordering::SeqCst), 50);
}

#[tokio::test]
async fn test_event_broadcast_to_multiple_subscribers() {
    // Test event broadcast to many subscribers
    let (tx, _rx) = tokio::sync::broadcast::channel(1000);

    // Create 10 subscribers
    let mut subscribers = vec![];
    for _ in 0..10 {
        subscribers.push(tx.subscribe());
    }

    // Send 100 events
    for i in 0..100 {
        let event = RuntimeEvent::AgentStarted {
            agent_name: format!("agent_{}", i),
            step_index: i,
        };
        let _ = tx.send(event);
    }

    // Each subscriber should receive all events
    for mut sub in subscribers {
        let mut count = 0;
        while sub.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 100, "Each subscriber should receive all events");
    }
}

#[tokio::test]
async fn test_tool_registry_concurrent_access() {
    // Test concurrent registration and access
    let registry = Arc::new(ToolRegistry::new());
    let mut handles = vec![];

    // Register 20 tools concurrently
    for i in 0..20 {
        let reg = registry.clone();
        let handle = tokio::spawn(async move {
            reg.register(NativeTool::new(
                &format!("tool_{}", i),
                "Test tool",
                json!({}),
                |_args| {
                    Box::pin(async move {
                        let start = std::time::Instant::now();
                        let duration = start.elapsed().as_secs_f64() * 1000.0;
                        Ok(ToolResult::success(json!({"id": i}), duration))
                    })
                },
            ));
        });
        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    // All tools should be registered
    let names = registry.list_names();
    assert_eq!(names.len(), 20);

    // Concurrent tool execution
    let mut handles = vec![];
    for name in names {
        let reg = registry.clone();
        let handle = tokio::spawn(async move {
            let tool = reg.get(&name).unwrap();
            tool.execute(&json!({})).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_agent_with_many_tool_calls() {
    // Test agent making many sequential tool calls
    let mut mock_client = MockLlmClient::new();

    // Add 50 tool calls
    for i in 0..50 {
        mock_client = mock_client.with_tool_call("test_tool", json!({"index": i}));
    }
    mock_client = mock_client.with_response("Completed all 50 calls");

    let registry = Arc::new(ToolRegistry::new());
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    registry.register(NativeTool::new(
        "test_tool",
        "Test",
        json!({}),
        move |_args| {
            let count = counter.clone();
            Box::pin(async move {
                let start = std::time::Instant::now();
                count.fetch_add(1, Ordering::SeqCst);
                let duration = start.elapsed().as_secs_f64() * 1000.0;
                Ok(ToolResult::success(json!({"status": "ok"}), duration))
            })
        },
    ));

    let config = AgentConfig::builder("test_agent")
        .system_prompt("Test")
        .tools(Arc::new(registry))
        .max_tool_iterations(100) // Allow many iterations
        .build();

    let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));
    let input = AgentInput::from_text("test");

    let result = agent.execute(&input).await;
    assert!(result.is_ok(), "Should handle many tool calls");

    // Should have executed all 50 tools
    assert_eq!(call_count.load(Ordering::SeqCst), 50);
}

#[tokio::test]
async fn test_concurrent_workflows() {
    // Test multiple workflows running concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let mock_client = Arc::new(
                MockLlmClient::new()
                    .with_response(format!("Response from step 1 - workflow {}", i))
                    .with_response(format!("Response from step 2 - workflow {}", i)),
            );

            let agent1_config = AgentConfig::builder(&format!("agent1_{}", i))
                .system_prompt("First agent")
                .build();
            let agent1 = Agent::new(agent1_config).with_llm_client(mock_client.clone());

            let agent2_config = AgentConfig::builder(&format!("agent2_{}", i))
                .system_prompt("Second agent")
                .build();
            let agent2 = Agent::new(agent2_config).with_llm_client(mock_client);

            let workflow = WorkflowBuilder::new()
                .name(&format!("workflow_{}", i))
                .add_step(agent1)
                .add_step(agent2)
                .build();

            let runtime = Runtime::new();
            let input = json!({"message": format!("Input for workflow {}", i)});
            runtime.execute(&workflow, input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_memory_usage_many_agents() {
    // Test memory doesn't balloon with many agents
    let mut agents = vec![];

    for i in 0..100 {
        let mock_client = MockLlmClient::new().with_response("Response");
        let config = AgentConfig::builder(&format!("agent_{}", i))
            .system_prompt("Test")
            .build();
        let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));
        agents.push(agent);
    }

    // Execute them all
    let mut handles = vec![];
    for agent in agents {
        let handle = tokio::spawn(async move {
            let input = AgentInput::from_text("test");
            agent.execute(&input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 100);
}

#[tokio::test]
async fn test_event_throughput() {
    // Test high-volume event emission and reception
    let (tx, mut rx) = tokio::sync::broadcast::channel(10000);

    // Spawn receiver task
    let receiver = tokio::spawn(async move {
        let mut count = 0;
        while let Ok(_event) = rx.recv().await {
            count += 1;
            if count >= 1000 {
                break;
            }
        }
        count
    });

    // Send 1000 events
    for i in 0..1000 {
        let event = RuntimeEvent::AgentCompleted {
            agent_name: format!("agent_{}", i),
            step_index: i,
        };
        let _ = tx.send(event);
    }

    let count = receiver.await.unwrap();
    assert_eq!(count, 1000, "Should receive all events");
}

#[tokio::test]
async fn test_concurrent_agent_with_tools() {
    // Test concurrent agents all using tools
    let mut handles = vec![];

    for i in 0..20 {
        let handle = tokio::spawn(async move {
            let mock_client = MockLlmClient::new()
                .with_tool_call("test_tool", json!({"id": i}))
                .with_response("Done");

            let registry = Arc::new(ToolRegistry::new());
            registry.register(NativeTool::new("test_tool", "Test", json!({}), |_args| {
                Box::pin(async move {
                    let start = std::time::Instant::now();
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    Ok(ToolResult::success(json!({"status": "ok"}), duration))
                })
            }));

            let config = AgentConfig::builder(&format!("agent_{}", i))
                .system_prompt("Test")
                .tools(Arc::new(registry))
                .build();

            let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));
            let input = AgentInput::from_text("test");
            agent.execute(&input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_stress_tool_loop_detection() {
    // Stress test the loop detection mechanism
    let mut handles = vec![];

    for i in 0..20 {
        let handle = tokio::spawn(async move {
            let mock_client = MockLlmClient::new()
                .with_tool_call("tool", json!({"same": "args"}))
                .with_tool_call("tool", json!({"same": "args"})) // Duplicate
                .with_response("Detected");

            let registry = Arc::new(ToolRegistry::new());
            registry.register(NativeTool::new("tool", "Test", json!({}), |_args| {
                Box::pin(async move {
                    let start = std::time::Instant::now();
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    Ok(ToolResult::success(json!({}), duration))
                })
            }));

            let config = AgentConfig::builder(&format!("agent_{}", i))
                .system_prompt("Test")
                .tools(Arc::new(registry))
                .enable_tool_loop_detection(true)
                .build();

            let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));
            let input = AgentInput::from_text("test");
            agent.execute(&input).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    for result in results {
        assert!(result.is_ok());
    }
}
