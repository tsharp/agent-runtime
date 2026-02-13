use agent_runtime::{
    tool::{CalculatorTool, ToolRegistry}, AgentConfig, AgentStep, Runtime, SubWorkflowStep, TransformStep, Workflow,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Workflow Composition Demo ===\n");

    // Define a reusable sub-workflow for data validation
    let validation_workflow_builder = || {
        let validate_step = TransformStep::new("validate_input".to_string(), |data| {
            let num = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            let is_valid = (0..=100).contains(&num);
            serde_json::json!({
                "value": num,
                "is_valid": is_valid,
                "validation_message": if is_valid {
                    "Value is within valid range"
                } else {
                    "Value is out of range (0-100)"
                }
            })
        });

        Workflow::builder().step(Box::new(validate_step)).build()
    };

    // Define a reusable sub-workflow for calculation
    let calculation_workflow_builder = || {
        let extract_step = TransformStep::new("extract_value".to_string(), |data| {
            serde_json::json!({
                "value": data.get("value").and_then(|v| v.as_i64()).unwrap_or(0)
            })
        });

        let calculate_step = TransformStep::new("calculate".to_string(), |data| {
            let val = data.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            serde_json::json!({
                "original": val,
                "doubled": val * 2,
                "squared": val * val
            })
        });

        let mut registry = ToolRegistry::new();
        registry.register(CalculatorTool);

        let agent = AgentConfig::builder("summarizer")
            .system_prompt("You summarize calculation results.")
            .tools(Arc::new(registry))
            .build();

        Workflow::builder()
            .step(Box::new(extract_step))
            .step(Box::new(calculate_step))
            .step(Box::new(AgentStep::new(agent)))
            .build()
    };

    // Main workflow that composes sub-workflows
    let main_workflow = Workflow::builder()
        .step(Box::new(SubWorkflowStep::new(
            "validation_pipeline".to_string(),
            validation_workflow_builder,
        )))
        .step(Box::new(SubWorkflowStep::new(
            "calculation_pipeline".to_string(),
            calculation_workflow_builder,
        )))
        .step(Box::new(TransformStep::new(
            "final_format".to_string(),
            |data| {
                serde_json::json!({
                    "result": data,
                    "processed_at": chrono::Utc::now().to_rfc3339()
                })
            },
        )))
        .initial_input(serde_json::json!({
            "value": 7,
            "source": "user_input"
        }))
        .build();

    println!("Main Workflow ID: {}", main_workflow.id);
    println!("Steps: {}", main_workflow.steps.len());
    println!("  1. Sub-Workflow: validation_pipeline");
    println!("  2. Sub-Workflow: calculation_pipeline");
    println!("  3. Transform: final_format\n");

    let runtime = Runtime::new();

    // Subscribe to events to see nested workflow events
    let mut event_receiver = runtime.event_stream().subscribe();

    let event_listener = tokio::spawn(async move {
        println!("📡 Event Monitor Active\n");
        while let Ok(event) = event_receiver.recv().await {
            let parent_info = if let Some(ref parent_id) = event.parent_workflow_id {
                format!(" [parent: {}]", &parent_id[..8])
            } else {
                String::new()
            };

            println!(
                "  [{}] {:?}{}",
                &event.workflow_id[..8],
                event.event_type,
                parent_info
            );
        }
    });

    println!("Executing main workflow...\n");
    let run = runtime.execute(main_workflow).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("\n=== Execution Complete ===");
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

    // Show event hierarchy
    println!("\n=== Event Hierarchy ===");
    let all_events = runtime.event_stream().all();

    // Group by workflow
    let mut workflows = std::collections::HashMap::new();
    for event in &all_events {
        workflows
            .entry(event.workflow_id.clone())
            .or_insert_with(Vec::new)
            .push(event);
    }

    println!("Total workflows executed: {}", workflows.len());
    for (wf_id, events) in &workflows {
        let parent_info = events
            .first()
            .and_then(|e| e.parent_workflow_id.as_ref())
            .map(|p| format!(" (child of {})", &p[..8]))
            .unwrap_or_default();

        println!(
            "  Workflow {}{}: {} events",
            &wf_id[..8],
            parent_info,
            events.len()
        );
    }

    event_listener.abort();

    println!("\n✅ Workflow composition working! Nested workflows executed successfully.");
}
