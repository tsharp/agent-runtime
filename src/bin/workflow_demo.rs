use agent_runtime::{
    llm::{ChatClient, LlamaClient},
    Agent, AgentConfig, AgentStep, EventType, FileLogger, Runtime, Workflow,
};
use std::fs;
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() {
    println!("=== Workflow Demo ===\n");
    
    // Create output directory
    fs::create_dir_all("output").expect("Failed to create output directory");
    
    // Create file logger
    let logger = FileLogger::new("output/workflow_demo.log")
        .expect("Failed to create log file");
    logger.log("=== Workflow Demo Started ===");

    // Create LLM client (insecure HTTPS for local dev)
    let llm_client: Arc<dyn ChatClient> =
        Arc::new(LlamaClient::insecure("https://192.168.91.57", "default"));

    println!("✓ LLM client configured (https://192.168.91.57 - insecure)\n");
    logger.log("LLM client configured");

    // Create agents
    let greeter = Agent::new(
        AgentConfig::builder("greeter")
            .system_prompt("You are a friendly greeter. Say hello and introduce yourself warmly.")
            .build(),
    )
    .with_llm_client(llm_client.clone());

    let analyzer = Agent::new(
        AgentConfig::builder("analyzer")
            .system_prompt("You are a thoughtful analyzer. Analyze the input and provide insights.")
            .build(),
    )
    .with_llm_client(llm_client.clone());

    let summarizer = Agent::new(
        AgentConfig::builder("summarizer")
            .system_prompt(
                "You are a concise summarizer. Summarize the conversation in 2-3 sentences.",
            )
            .build(),
    )
    .with_llm_client(llm_client.clone());

    println!("✓ Created 3 agents: greeter → analyzer → summarizer\n");

    // Build workflow
    let workflow = Workflow::builder()
        .step(Box::new(AgentStep::from_agent(
            greeter,
            "greeter".to_string(),
        )))
        .step(Box::new(AgentStep::from_agent(
            analyzer,
            "analyzer".to_string(),
        )))
        .step(Box::new(AgentStep::from_agent(
            summarizer,
            "summarizer".to_string(),
        )))
        .initial_input(serde_json::json!(
            "Hello! I'm interested in learning about AI agents."
        ))
        .build();

    println!("✓ Workflow built with 3 sequential steps\n");

    // Show mermaid diagram
    println!("Workflow Structure:");
    println!("{}\n", workflow.to_mermaid());

    // Create runtime
    let runtime = Runtime::new();

    // Subscribe to events in a separate task
    let mut event_receiver = runtime.event_stream().subscribe();
    let logger_for_events = logger.clone();
    let event_task = task::spawn(async move {
        println!("📡 Streaming Agent Responses\n");
        println!("{}", "=".repeat(60));

        let _current_agent: Option<String> = None;

        while let Ok(event) = event_receiver.recv().await {
            // Log all events to file
            logger_for_events.log_level(
                &format!("{:?}", event.event_type),
                &serde_json::to_string(&event.data).unwrap_or_default()
            );
            
            match event.event_type {
                EventType::AgentProcessing => {
                    if let Some(agent) = event.data.get("agent").and_then(|v| v.as_str()) {
                        println!("\n🤖 {} >", agent);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                }
                EventType::AgentLlmStreamChunk => {
                    if let Some(chunk) = event.data.get("chunk").and_then(|v| v.as_str()) {
                        print!("{}", chunk);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                }
                EventType::AgentLlmRequestCompleted => {
                    println!(); // New line after streaming completes
                }
                EventType::AgentLlmRequestFailed => {
                    if let Some(_agent) = event.data.get("agent").and_then(|v| v.as_str()) {
                        if let Some(error) = event.data.get("error").and_then(|v| v.as_str()) {
                            println!("\n   ❌ Error: {}", error);
                        }
                    }
                }
                EventType::WorkflowCompleted => {
                    println!("\n{}", "=".repeat(60));
                    println!("✅ Workflow Completed");
                    break;
                }
                EventType::WorkflowFailed => {
                    println!("\n{}", "=".repeat(60));
                    println!("❌ Workflow Failed");
                    break;
                }
                _ => {}
            }
        }
    });

    // Execute workflow
    println!("\n▶ Starting workflow execution...\n");
    logger.log("Starting workflow execution");

    let result = runtime.execute(workflow).await;
    
    logger.log(&format!("Workflow completed. Steps: {}", result.steps.len()));

    // Wait for event task to finish
    let _ = event_task.await;

    // Show final results
    println!("\n{}", "=".repeat(60));
    println!("\n📊 Final Results\n");

    if let Some(output) = &result.final_output {
        println!("Output:");
        if let Some(response) = output.get("response") {
            match response {
                serde_json::Value::String(s) => println!("{}\n", s),
                _ => println!("{}\n", serde_json::to_string_pretty(response).unwrap()),
            }
        } else {
            println!("{}\n", serde_json::to_string_pretty(output).unwrap());
        }
    }

    // Write result to file
    let result_json = serde_json::to_string_pretty(&result).unwrap();
    fs::write("output/workflow_demo_result.json", result_json).expect("Failed to write result file");
    println!("💾 Results written to output/");
    println!("   - workflow_demo.log (debug log)");
    println!("   - workflow_demo_result.json (execution result)\n");
    logger.log("Results written to output/workflow_demo_result.json");

    println!("Steps executed: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        let msg = format!(
            "{}. {} ({}) - {}ms",
            i + 1,
            step.step_name,
            step.step_type,
            step.execution_time_ms.unwrap_or(0)
        );
        println!("  {}", msg);
        logger.log(&msg);
    }
    
    logger.log("=== Workflow Demo Completed ===");
}
