/// Async Event Streaming Demonstration
///
/// This demo shows the v0.3.0 unified event system with artificial delays
/// to make the async event sequence clearly visible.
///
/// Features demonstrated:
/// - Workflow lifecycle events (Started, Completed)
/// - WorkflowStep events for each step
/// - Complete event timestamps showing async behavior
/// - 500ms artificial delays to make sequence observable
///
/// Run with: cargo run --bin async_events_demo

use agent_runtime::event::{Event, EventScope, EventType};
use agent_runtime::EventStream;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

/// Event monitor that displays events in real-time with formatting
async fn monitor_events(mut rx: tokio::sync::broadcast::Receiver<Event>) {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    EVENT STREAM MONITOR                       ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let start_time = std::time::Instant::now();

    while let Ok(event) = rx.recv().await {
        let elapsed = start_time.elapsed().as_secs_f64();
        let scope = event.scope.clone();
        let event_type = event.event_type.clone();

        match (scope, event_type) {
            (EventScope::Workflow, EventType::Started) => {
                println!(
                    "🚀 [{:>6.2}s] Workflow Started: {}",
                    elapsed, event.component_id
                );
                println!("   └─ Status: {:?}", event.status);
            }
            (EventScope::Workflow, EventType::Completed) => {
                println!(
                    "\n✅ [{:>6.2}s] Workflow Completed: {}",
                    elapsed, event.component_id
                );
                if let Some(duration) = event.data.get("duration_ms") {
                    println!("   └─ Total Duration: {}ms", duration);
                }
                println!("\n{}", "═".repeat(65));
                break; // Exit after workflow completes
            }

            (EventScope::WorkflowStep, EventType::Started) => {
                println!(
                    "\n▶  [{:>6.2}s] Step Started: {}",
                    elapsed, event.component_id
                );
                if let Some(step_type) = event.data.get("step_type") {
                    println!("   └─ Type: {}", step_type);
                }
            }
            (EventScope::WorkflowStep, EventType::Completed) => {
                println!(
                    "⏹  [{:>6.2}s] Step Completed: {}",
                    elapsed, event.component_id
                );
                if let Some(duration) = event.data.get("duration_ms") {
                    println!("   └─ Duration: {}ms", duration);
                }
            }

            (EventScope::System, EventType::Progress) => {
                println!(
                    "   ⚙  [{:>6.2}s] System: {}",
                    elapsed,
                    event.message.as_deref().unwrap_or("Progress")
                );
            }

            (_, EventType::Failed) => {
                eprintln!(
                    "❌ [{:>6.2}s] {:?} Failed: {}",
                    elapsed, event.scope, event.component_id
                );
                if let Some(msg) = &event.message {
                    eprintln!("   └─ Error: {}", msg);
                }
                break; // Exit on failure
            }

            _ => {
                // Other events
                println!(
                    "   · [{:>6.2}s] {:?}::{:?} ({})",
                    elapsed, event.scope, event.event_type, event.component_id
                );
            }
        }

        io::stdout().flush().unwrap();
    }
}

/// Simulates a workflow step with artificial delay
async fn simulate_step(
    stream: &EventStream,
    workflow_id: &str,
    step_num: usize,
    delay_ms: u64,
) {
    use agent_runtime::event::{ComponentStatus, EventScope, EventType};

    let component_id = format!("demo_workflow:step:{}", step_num);

    // Emit step started event
    stream
        .append(
            EventScope::WorkflowStep,
            EventType::Started,
            component_id.clone(),
            ComponentStatus::Running,
            workflow_id.to_string(),
            None,
            serde_json::json!({
                "step_type": "transform",
                "step_number": step_num
            }),
        )
        .await;

    // Simulate work with delay
    sleep(Duration::from_millis(delay_ms)).await;

    // Emit step completed event
    stream
        .append(
            EventScope::WorkflowStep,
            EventType::Completed,
            component_id,
            ComponentStatus::Completed,
            workflow_id.to_string(),
            None,
            serde_json::json!({
                "step_type": "transform",
                "duration_ms": delay_ms,
                "output": format!("Step {} result", step_num)
            }),
        )
        .await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║         ASYNC EVENT STREAMING DEMONSTRATION (v0.3.0)          ║");
    println!("║                                                               ║");
    println!("║  This demo shows the unified event system with artificial    ║");
    println!("║  delays to make the event sequence clearly visible.          ║");
    println!("║                                                               ║");
    println!("║  • 10 workflow steps                                          ║");
    println!("║  • 500ms delay per step                                       ║");
    println!("║  • Real-time async event emission                             ║");
    println!("║  • Complete lifecycle tracking                                ║");
    println!("║  • Unified Scope × Type × Status pattern                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    const NUM_STEPS: usize = 10;
    const STEP_DELAY_MS: u64 = 500;

    println!("✓ Creating workflow with {} steps", NUM_STEPS);
    println!("✓ Each step has {}ms artificial delay", STEP_DELAY_MS);
    println!(
        "✓ Total expected runtime: ~{:.1}s",
        (NUM_STEPS as f64 * STEP_DELAY_MS as f64) / 1000.0
    );

    // Create event stream
    let stream = EventStream::new();
    let rx = stream.subscribe();

    // Spawn event monitor in background
    tokio::spawn(monitor_events(rx));

    // Small delay to let monitor start
    sleep(Duration::from_millis(100)).await;

    println!("\n⏳ Starting workflow execution...\n");

    let workflow_id = "demo_workflow";
    let start_time = std::time::Instant::now();

    // Emit workflow started event
    stream
        .append(
            EventScope::Workflow,
            EventType::Started,
            workflow_id.to_string(),
            agent_runtime::event::ComponentStatus::Running,
            workflow_id.to_string(),
            None,
            serde_json::json!({
                "num_steps": NUM_STEPS,
                "input": "Demonstration input data"
            }),
        )
        .await;

    // Execute each step sequentially
    for step_num in 0..NUM_STEPS {
        simulate_step(&stream, workflow_id, step_num, STEP_DELAY_MS).await;
    }

    let total_duration_ms = start_time.elapsed().as_millis() as u64;

    // Emit workflow completed event
    stream
        .append(
            EventScope::Workflow,
            EventType::Completed,
            workflow_id.to_string(),
            agent_runtime::event::ComponentStatus::Completed,
            workflow_id.to_string(),
            None,
            serde_json::json!({
                "steps_completed": NUM_STEPS,
                "duration_ms": total_duration_ms,
                "output": "All steps completed successfully"
            }),
        )
        .await;

    // Wait for final events to be displayed
    sleep(Duration::from_millis(500)).await;

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                      DEMONSTRATION COMPLETE                   ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");
    println!("✓ {} steps executed", NUM_STEPS);
    println!("✓ Total time: {:.2}s", total_duration_ms as f64 / 1000.0);
    println!("✓ All events emitted asynchronously");
    println!("\nKey observations:");
    println!("  • Events appear in real-time as work progresses");
    println!("  • Timestamps show async execution (no blocking)");
    println!("  • Unified event pattern (Scope × Type × Status)");
    println!("  • Component IDs follow enforced format (workflow:step:N)");
    println!("\nTry this with a real workflow:");
    println!("  cargo run --bin workflow_demo");
    println!();

    Ok(())
}
