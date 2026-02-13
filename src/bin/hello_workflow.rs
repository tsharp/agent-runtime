use agent_runtime::{
    AgentConfig, Runtime, Workflow, tool::{EchoTool, CalculatorTool}
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Agent Workflow Runtime - Hello World Example ===\n");
    
    // Create tools
    let echo_tool = Arc::new(EchoTool);
    let calculator_tool = Arc::new(CalculatorTool);
    
    // Build agents
    let greeter = AgentConfig::builder("greeter")
        .system_prompt("You are a friendly greeter. Say hello to the user.")
        .tool(echo_tool.clone())
        .build();
    
    let calculator = AgentConfig::builder("calculator")
        .system_prompt("You are a calculator. Perform mathematical operations.")
        .tool(calculator_tool)
        .build();
    
    let summarizer = AgentConfig::builder("summarizer")
        .system_prompt("You summarize the results from previous steps.")
        .build();
    
    // Build workflow
    let workflow = Workflow::builder()
        .agent(greeter)
        .agent(calculator)
        .agent(summarizer)
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
    println!("Agents: {}\n", workflow.agents.len());
    
    // Create runtime and execute
    let mut runtime = Runtime::new();
    
    println!("Executing workflow...\n");
    let run = runtime.execute(workflow).await;
    
    // Print results
    println!("=== Execution Complete ===");
    println!("Status: {:?}", run.state);
    println!("Steps executed: {}\n", run.steps.len());
    
    for step in &run.steps {
        println!("Step {}: {}", step.step_index, step.agent_name);
        println!("  Input: {}", serde_json::to_string_pretty(&step.input).unwrap());
        if let Some(ref output) = step.output {
            println!("  Output: {}", serde_json::to_string_pretty(output).unwrap());
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
    
    // Print event stream
    println!("=== Event Stream ({} events) ===", runtime.event_stream().len());
    for event in runtime.event_stream().all() {
        println!(
            "[offset:{}] {:?} - {}",
            event.offset,
            event.event_type,
            serde_json::to_string(&event.data).unwrap()
        );
    }
    
    println!("\n=== Snapshot (JSON) ===");
    let snapshot = serde_json::json!({
        "run": run,
        "events": runtime.event_stream().all(),
    });
    println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
}
