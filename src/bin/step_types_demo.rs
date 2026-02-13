use agent_runtime::{
    tool::{CalculatorTool, ToolRegistry}, AgentConfig, AgentStep, ConditionalStep, Runtime, TransformStep, Workflow,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Step Types Demo ===\n");

    // Step 1: Transform - Extract a field
    let extract_step = TransformStep::new("extract_number".to_string(), |data| {
        serde_json::json!({
            "value": data.get("number").and_then(|v| v.as_i64()).unwrap_or(0)
        })
    });

    // Step 2: Conditional - Check if number is positive
    let positive_transform = TransformStep::new("positive_handler".to_string(), |data| {
        let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        serde_json::json!({
            "value": val,
            "status": "positive",
            "doubled": val * 2
        })
    });

    let negative_transform = TransformStep::new("negative_handler".to_string(), |data| {
        let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        serde_json::json!({
            "value": val,
            "status": "negative or zero",
            "absolute": val.abs()
        })
    });

    let conditional_step = ConditionalStep::new(
        "check_positive".to_string(),
        |data| {
            data.get("value")
                .and_then(|v| v.as_i64())
                .map(|n| n > 0)
                .unwrap_or(false)
        },
        Box::new(positive_transform),
        Box::new(negative_transform),
    );

    // Step 3: Agent - Summarize the result
    let mut registry = ToolRegistry::new();
    registry.register(CalculatorTool);
    
    let agent = AgentConfig::builder("summarizer")
        .system_prompt("You summarize numerical results.")
        .tools(Arc::new(registry))
        .build();

    // Build workflow combining different step types
    let workflow = Workflow::builder()
        .step(Box::new(extract_step))
        .step(Box::new(conditional_step))
        .step(Box::new(AgentStep::new(agent)))
        .initial_input(serde_json::json!({
            "number": 42,
            "description": "Test number"
        }))
        .build();

    println!("Workflow ID: {}", workflow.id);
    println!("Steps: {}", workflow.steps.len());
    println!("  1. Transform (extract_number)");
    println!("  2. Conditional (check_positive)");
    println!("  3. Agent (summarizer)\n");

    let runtime = Runtime::new();

    println!("Executing workflow...\n");
    let run = runtime.execute(workflow).await;

    println!("=== Execution Complete ===");
    println!("Status: {:?}\n", run.state);

    for step in &run.steps {
        println!(
            "Step {}: {} [{}]",
            step.step_index, step.step_name, step.step_type
        );
        println!("  Input: {}", serde_json::to_string(&step.input).unwrap());
        if let Some(ref output) = step.output {
            println!("  Output: {}", serde_json::to_string(&output).unwrap());
        }
        println!("  Time: {}ms\n", step.execution_time_ms.unwrap_or(0));
    }

    if let Some(ref final_output) = run.final_output {
        println!("=== Final Output ===");
        println!("{}", serde_json::to_string_pretty(final_output).unwrap());
    }

    println!("\n=== Now testing with negative number ===\n");

    // Test with negative number
    let extract_step2 = TransformStep::new("extract_number".to_string(), |data| {
        serde_json::json!({
            "value": data.get("number").and_then(|v| v.as_i64()).unwrap_or(0)
        })
    });

    let positive_transform2 = TransformStep::new("positive_handler".to_string(), |data| {
        let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        serde_json::json!({
            "value": val,
            "status": "positive",
            "doubled": val * 2
        })
    });

    let negative_transform2 = TransformStep::new("negative_handler".to_string(), |data| {
        let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
        serde_json::json!({
            "value": val,
            "status": "negative or zero",
            "absolute": val.abs()
        })
    });

    let conditional_step2 = ConditionalStep::new(
        "check_positive".to_string(),
        |data| {
            data.get("value")
                .and_then(|v| v.as_i64())
                .map(|n| n > 0)
                .unwrap_or(false)
        },
        Box::new(positive_transform2),
        Box::new(negative_transform2),
    );

    let agent2 = AgentConfig::builder("summarizer")
        .system_prompt("You summarize numerical results.")
        .build();

    let workflow2 = Workflow::builder()
        .step(Box::new(extract_step2))
        .step(Box::new(conditional_step2))
        .step(Box::new(AgentStep::new(agent2)))
        .initial_input(serde_json::json!({
            "number": -15,
            "description": "Negative test"
        }))
        .build();

    let run2 = runtime.execute(workflow2).await;

    println!("Status: {:?}\n", run2.state);

    for step in &run2.steps {
        println!(
            "Step {}: {} [{}]",
            step.step_index, step.step_name, step.step_type
        );
        if let Some(ref output) = step.output {
            println!("  Output: {}", serde_json::to_string(&output).unwrap());
        }
    }

    if let Some(ref final_output) = run2.final_output {
        println!("\n=== Final Output ===");
        println!("{}", serde_json::to_string_pretty(final_output).unwrap());
    }

    println!("\n✅ Step abstraction working! Multiple step types demonstrated.");
}
