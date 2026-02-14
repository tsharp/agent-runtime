use agent_runtime::{
    tool::{CalculatorTool, EchoTool, ToolRegistry},
    AgentConfig, AgentStep, Runtime, Workflow,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Agent Workflow Runtime - Hello World Example ===\n");

    // Create tools
    let mut echo_registry = ToolRegistry::new();
    echo_registry.register(EchoTool);
    let echo_registry = Arc::new(echo_registry);

    let mut calc_registry = ToolRegistry::new();
    calc_registry.register(CalculatorTool);
    let calc_registry = Arc::new(calc_registry);

    // Build agents
    let greeter = AgentConfig::builder("greeter")
        .system_prompt("You are a friendly greeter. Say hello to the user.")
        .tools(echo_registry)
        .build();

    let calculator = AgentConfig::builder("calculator")
        .system_prompt("You are a calculator. Perform mathematical operations.")
        .tools(calc_registry)
        .build();

    let summarizer = AgentConfig::builder("summarizer")
        .system_prompt("You summarize the results from previous steps.")
        .build();

    // Build workflow with steps (NEW API)
    let workflow = Workflow::builder()
        .step(Box::new(AgentStep::new(greeter)))
        .step(Box::new(AgentStep::new(calculator)))
        .step(Box::new(AgentStep::new(summarizer)))
        .initial_input(serde_json::json!({
            "user_name": "World",
            "calculation": {
                "operation": "add",
                "a": 10,
                "b": 32
            }
        }))
        .build();

    println!("Workflow ID: {}", workflow.id);
    println!("Steps: {}\n", workflow.steps.len());

    // Create runtime and execute
    let runtime = Runtime::new();

    // Subscribe to event stream (simulate real-time listener)
    let mut event_receiver = runtime.event_stream().subscribe();

    // Spawn task to listen to events in real-time
    let event_listener = tokio::spawn(async move {
        println!("📡 Real-time Event Listener Active\n");
        let mut count = 0;
        while let Ok(event) = event_receiver.recv().await {
            count += 1;
            println!(
                "  [LIVE] Event #{}: {:?} @ offset {}",
                count, event.event_type, event.offset
            );
        }
    });

    println!("Executing workflow...\n");
    let run = runtime.execute(workflow).await;

    // Give event listener a moment to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Print results
    println!("\n=== Execution Complete ===");
    println!("Status: {:?}", run.state);
    println!("Steps executed: {}\n", run.steps.len());

    for step in &run.steps {
        println!(
            "Step {}: {} ({})",
            step.step_index, step.step_name, step.step_type
        );
        println!(
            "  Input: {}",
            serde_json::to_string_pretty(&step.input).unwrap()
        );
        if let Some(ref output) = step.output {
            println!(
                "  Output: {}",
                serde_json::to_string_pretty(output).unwrap()
            );
        }
        if let Some(time) = step.execution_time_ms {
            println!("  Execution time: {}ms", time);
        }
        println!();
    }

    if let Some(ref final_output) = run.final_output {
        println!("=== Final Output ===");
        println!("{}", serde_json::to_string_pretty(final_output).unwrap());
        println!();
    }

    // Print event stream from history
    println!(
        "=== Event History ({} events) ===",
        runtime.event_stream().len()
    );
    for event in runtime.event_stream().all() {
        println!(
            "[offset:{}] {:?} - {}",
            event.offset,
            event.event_type,
            serde_json::to_string(&event.data).unwrap()
        );
    }

    // Demonstrate offset-based replay
    println!("\n=== Replay from Offset 5 ===");
    let replayed_events = runtime.events_from_offset(5);
    println!("Replaying {} events:", replayed_events.len());
    for event in &replayed_events {
        println!("  [offset:{}] {:?}", event.offset, event.event_type);
    }

    println!("\n=== Snapshot (JSON) ===");
    let snapshot = serde_json::json!({
        "run": run,
        "events": runtime.event_stream().all(),
        "total_events": runtime.event_stream().len(),
        "current_offset": runtime.event_stream().current_offset(),
    });
    println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());

    // Clean up event listener
    event_listener.abort();
}
