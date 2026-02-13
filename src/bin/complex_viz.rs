use agent_runtime::{
    AgentConfig, AgentStep, ConditionalStep, Runtime, SubWorkflowStep, TransformStep, Workflow,
};

#[tokio::main]
async fn main() {
    println!("=== Complex Workflow Visualization Demo ===\n");

    // Build deeply nested workflow with multiple branches

    // Inner sub-workflow: Data validation pipeline
    let validation_pipeline = || {
        Workflow::builder()
            .step(Box::new(TransformStep::new(
                "parse_input".to_string(),
                |data| {
                    serde_json::json!({
                        "parsed": true,
                        "data": data
                    })
                },
            )))
            .step(Box::new(ConditionalStep::new(
                "check_schema".to_string(),
                |_data| true, // Always valid for demo
                Box::new(TransformStep::new("schema_valid".to_string(), |data| {
                    serde_json::json!({
                        "validation": "passed",
                        "data": data
                    })
                })),
                Box::new(TransformStep::new("schema_invalid".to_string(), |_data| {
                    serde_json::json!({
                        "validation": "failed",
                        "error": "Invalid schema"
                    })
                })),
            )))
            .build()
    };

    // Nested sub-workflow: Processing pipeline
    let processing_pipeline = move || {
        Workflow::builder()
            .step(Box::new(AgentStep::new(
                AgentConfig::builder("processor")
                    .system_prompt("Process the data")
                    .build(),
            )))
            .step(Box::new(SubWorkflowStep::new(
                "validate_results".to_string(),
                validation_pipeline, // Nested 2 levels deep!
            )))
            .build()
    };

    // Main workflow with complex branching
    let main_workflow = Workflow::builder()
        .step(Box::new(AgentStep::new(
            AgentConfig::builder("input_handler")
                .system_prompt("Handle initial input")
                .build(),
        )))
        .step(Box::new(ConditionalStep::new(
            "route_by_type".to_string(),
            |data| {
                data.get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "premium")
                    .unwrap_or(false)
            },
            // Premium path - with nested workflow
            Box::new(SubWorkflowStep::new(
                "premium_processing".to_string(),
                processing_pipeline,
            )),
            // Standard path - simple transform
            Box::new(TransformStep::new(
                "standard_processing".to_string(),
                |data| {
                    serde_json::json!({
                        "tier": "standard",
                        "data": data
                    })
                },
            )),
        )))
        .step(Box::new(ConditionalStep::new(
            "quality_check".to_string(),
            |data| {
                data.get("validation")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "passed")
                    .unwrap_or(true)
            },
            Box::new(TransformStep::new("publish".to_string(), |data| {
                serde_json::json!({
                    "status": "published",
                    "data": data
                })
            })),
            Box::new(TransformStep::new("reject".to_string(), |data| {
                serde_json::json!({
                    "status": "rejected",
                    "data": data
                })
            })),
        )))
        .step(Box::new(AgentStep::new(
            AgentConfig::builder("finalizer")
                .system_prompt("Finalize the output")
                .build(),
        )))
        .initial_input(serde_json::json!({
            "type": "premium",
            "user_id": 12345,
            "data": "important payload"
        }))
        .build();

    println!("Workflow ID: {}\n", main_workflow.id);
    println!(
        "Total steps in main workflow: {}\n",
        main_workflow.steps.len()
    );

    // Generate the diagram
    println!("=== Complex Workflow Structure (Mermaid) ===\n");
    let mermaid = main_workflow.to_mermaid();
    println!("{}", mermaid);
    println!();

    // Save to file
    std::fs::write("complex_workflow.mmd", mermaid.clone()).expect("Failed to write diagram");

    println!("=== Diagram Saved ===");
    println!("  - complex_workflow.mmd");
    println!();
    println!("View at: https://mermaid.live/");
    println!();

    println!("=== Key Features Shown ===");
    println!("  ✓ Conditional branching (TRUE/FALSE paths)");
    println!("  ✓ Sub-workflow expansion (inline visualization)");
    println!("  ✓ Nested sub-workflows (2 levels deep)");
    println!("  ✓ Branch convergence points");
    println!("  ✓ Multiple step types (Agent, Transform, Conditional, SubWorkflow)");
    println!("  ✓ Different node shapes per type");
    println!();

    // Execute to show it works
    println!("=== Executing Workflow ===");
    let runtime = Runtime::new();
    let run = runtime.execute(main_workflow).await;

    println!("Status: {:?}", run.state);
    println!("Steps executed: {}", run.steps.len());
    println!("\n✅ Complex visualization complete!");
}
