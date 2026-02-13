use agent_runtime::{
    AgentConfig, Runtime, Workflow, AgentStep,
    tool::EchoTool,
    event::EventType,
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Multi-Subscriber Event Stream Demo ===\n");
    
    // Create a simple workflow
    let agent = AgentConfig::builder("demo_agent")
        .system_prompt("Demo agent for testing event streams.")
        .tool(Arc::new(EchoTool))
        .build();
    
    let workflow = Workflow::builder()
        .step(Box::new(AgentStep::new(agent)))
        .initial_input(serde_json::json!({"test": "data"}))
        .build();
    
    let runtime = Runtime::new();
    
    // Create multiple subscribers
    let mut subscriber1 = runtime.event_stream().subscribe();
    let mut subscriber2 = runtime.event_stream().subscribe();
    let mut subscriber3 = runtime.event_stream().subscribe();
    
    println!("Started 3 independent subscribers\n");
    
    // Subscriber 1: Logs all events
    let logger = tokio::spawn(async move {
        println!("[Logger] Started");
        while let Ok(event) = subscriber1.recv().await {
            println!("[Logger] {:?} @ offset {}", event.event_type, event.offset);
        }
    });
    
    // Subscriber 2: Filters for workflow events only
    let workflow_monitor = tokio::spawn(async move {
        println!("[Workflow Monitor] Started");
        while let Ok(event) = subscriber2.recv().await {
            if matches!(
                event.event_type,
                EventType::WorkflowStarted
                    | EventType::WorkflowCompleted
                    | EventType::WorkflowFailed
            ) {
                println!(
                    "[Workflow Monitor] 🔔 {:?} - {}",
                    event.event_type,
                    serde_json::to_string(&event.data).unwrap()
                );
            }
        }
    });
    
    // Subscriber 3: Collects metrics
    let metrics_collector = tokio::spawn(async move {
        println!("[Metrics] Started");
        let mut total_events = 0;
        let mut agent_events = 0;
        
        while let Ok(event) = subscriber3.recv().await {
            total_events += 1;
            
            if matches!(
                event.event_type,
                EventType::AgentInitialized
                    | EventType::AgentCompleted
                    | EventType::AgentFailed
            ) {
                agent_events += 1;
            }
            
            // Print periodic summary
            if event.event_type == EventType::WorkflowCompleted {
                println!(
                    "[Metrics] 📊 Total: {}, Agent-related: {}",
                    total_events, agent_events
                );
            }
        }
    });
    
    println!("Executing workflow...\n");
    let run = runtime.execute(workflow).await;
    
    // Give subscribers time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n=== Workflow Complete ===");
    println!("Status: {:?}", run.state);
    println!("Total events in history: {}", runtime.event_stream().len());
    
    // Demonstrate replay for a late subscriber
    println!("\n=== Late Subscriber (Replay Demo) ===");
    println!("A new subscriber connecting after workflow completion...");
    
    let historical_events = runtime.events_from_offset(0);
    println!("Replaying {} historical events:", historical_events.len());
    for event in historical_events.iter().take(5) {
        println!("  {:?} @ offset {}", event.event_type, event.offset);
    }
    println!("  ... and {} more events", historical_events.len().saturating_sub(5));
    
    // Demonstrate partial replay
    println!("\n=== Partial Replay from Offset 3 ===");
    let partial_events = runtime.events_from_offset(3);
    println!("Replaying {} events from offset 3:", partial_events.len());
    for event in &partial_events {
        println!("  [{}] {:?}", event.offset, event.event_type);
    }
    
    // Clean up
    logger.abort();
    workflow_monitor.abort();
    metrics_collector.abort();
    
    println!("\n✅ Demo complete - Event broadcasting working!");
}
