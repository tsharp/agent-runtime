use agent_runtime::{
    AgentConfig, Runtime, Workflow, AgentStep, TransformStep, ConditionalStep, SubWorkflowStep,
    tool::CalculatorTool,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Mermaid Diagram Visualization Demo ===\n");
    
    // Build a complex workflow for visualization
    let greeter = AgentConfig::builder("greeter")
        .system_prompt("Greet the user")
        .build();
    
    let calculator = AgentConfig::builder("calculator")
        .system_prompt("Perform calculations")
        .tool(Arc::new(CalculatorTool))
        .build();
    
    let extract_transform = TransformStep::new(
        "extract_value".to_string(),
        |data| {
            serde_json::json!({
                "value": data.get("number").and_then(|v| v.as_i64()).unwrap_or(0)
            })
        },
    );
    
    let positive_handler = TransformStep::new(
        "positive_branch".to_string(),
        |data| {
            let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            serde_json::json!({ "result": val * 2, "status": "positive" })
        },
    );
    
    let negative_handler = TransformStep::new(
        "negative_branch".to_string(),
        |data| {
            let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            serde_json::json!({ "result": val.abs(), "status": "negative" })
        },
    );
    
    let conditional = ConditionalStep::new(
        "check_sign".to_string(),
        |data| data.get("value").and_then(|v| v.as_i64()).map(|n| n > 0).unwrap_or(false),
        Box::new(positive_handler),
        Box::new(negative_handler),
    );
    
    // Sub-workflow
    let sub_workflow_builder = || {
        Workflow::builder()
            .step(Box::new(TransformStep::new(
                "validate".to_string(),
                |data| {
                    serde_json::json!({
                        "validated": true,
                        "data": data
                    })
                },
            )))
            .build()
    };
    
    let workflow = Workflow::builder()
        .step(Box::new(AgentStep::new(greeter)))
        .step(Box::new(extract_transform))
        .step(Box::new(conditional))
        .step(Box::new(SubWorkflowStep::new(
            "validation_pipeline".to_string(),
            sub_workflow_builder,
        )))
        .step(Box::new(AgentStep::new(calculator)))
        .initial_input(serde_json::json!({
            "number": 42,
            "user": "Alice"
        }))
        .build();
    
    println!("Workflow ID: {}\n", workflow.id);
    
    // Generate Mermaid diagram BEFORE execution
    println!("=== Workflow Structure (Mermaid) ===\n");
    let mermaid_definition = workflow.to_mermaid();
    println!("{}", mermaid_definition);
    println!();
    
    // Execute the workflow
    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;
    
    println!("=== Execution Complete ===");
    println!("Status: {:?}", run.state);
    println!("Steps executed: {}\n", run.steps.len());
    
    // Generate Mermaid diagram AFTER execution (with results)
    println!("=== Workflow Execution Results (Mermaid) ===\n");
    let mermaid_results = run.to_mermaid_with_results();
    println!("{}", mermaid_results);
    println!();
    
    // Save to file
    std::fs::write("workflow_structure.mmd", mermaid_definition.clone())
        .expect("Failed to write structure diagram");
    std::fs::write("workflow_results.mmd", mermaid_results.clone())
        .expect("Failed to write results diagram");
    
    println!("=== Diagrams Saved ===");
    println!("  - workflow_structure.mmd (structure only)");
    println!("  - workflow_results.mmd (with execution results)");
    println!();
    println!("View online at: https://mermaid.live/");
    println!("Or in VS Code with Mermaid extension");
    println!();
    
    // Show how to render in markdown
    println!("=== Markdown Usage ===");
    println!("```mermaid");
    for line in mermaid_definition.lines().take(10) {
        println!("{}", line);
    }
    println!("...");
    println!("```");
    
    println!("\n✅ Mermaid visualization complete!");
}
