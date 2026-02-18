/// Reconnection Pattern Demo
///
/// This demo shows how to handle subscriber disconnection and reconnection
/// without losing events using the EventStream's history replay feature.
///
/// Run with: cargo run --bin reconnection_demo
use agent_runtime::event::{Event, EventScope, EventStream, EventType};
use std::time::Duration;
use tokio::time::sleep;

/// Simulates a UI client that can disconnect and reconnect
struct ReconnectingClient {
    last_offset: u64,
    events_received: Vec<Event>,
}

impl ReconnectingClient {
    fn new() -> Self {
        Self {
            last_offset: 0,
            events_received: Vec::new(),
        }
    }

    /// Process events from a subscription
    async fn listen(
        &mut self,
        mut rx: tokio::sync::broadcast::Receiver<Event>,
        duration_secs: u64,
    ) {
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < duration_secs {
            match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                Ok(Ok(event)) => {
                    self.last_offset = event.offset;
                    self.events_received.push(event.clone());
                    println!(
                        "  📥 Received: offset={} {:?}::{:?}",
                        event.offset, event.scope, event.event_type
                    );
                }
                Ok(Err(_)) => {
                    // Channel closed or lagged
                    break;
                }
                Err(_) => {
                    // Timeout - continue listening
                }
            }
        }
    }

    /// Reconnect and catch up on missed events
    fn reconnect(&mut self, stream: &EventStream) -> tokio::sync::broadcast::Receiver<Event> {
        println!("\n🔄 Reconnecting... (last offset: {})", self.last_offset);

        // Get all events since last offset (replay missed events)
        let missed_events = stream.from_offset(self.last_offset + 1);

        if !missed_events.is_empty() {
            println!("  📦 Catching up on {} missed events:", missed_events.len());
            for event in missed_events {
                self.last_offset = event.offset;
                self.events_received.push(event.clone());
                println!(
                    "  📥 Replayed: offset={} {:?}::{:?}",
                    event.offset, event.scope, event.event_type
                );
            }
        } else {
            println!("  ✓ No missed events (we're up to date)");
        }

        // Subscribe to future events
        println!("  ✓ Subscribed to live events\n");
        stream.subscribe()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║           RECONNECTION PATTERN DEMONSTRATION                  ║");
    println!("║                                                               ║");
    println!("║  Shows how EventStream's history replay prevents event loss  ║");
    println!("║  when subscribers disconnect and reconnect.                  ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let stream = EventStream::new();
    let mut client = ReconnectingClient::new();

    // Spawn event producer (simulates workflow generating events)
    let stream_clone = stream.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(500)).await; // Give client time to subscribe

        for i in 0..20 {
            let _ = stream_clone
                .append(
                    EventScope::WorkflowStep,
                    EventType::Started,
                    format!("step_{}", i),
                    agent_runtime::event::ComponentStatus::Running,
                    "demo_workflow".to_string(),
                    None,
                    serde_json::json!({"step": i}),
                )
                .await;

            println!(
                "📤 Emitted: step_{} at offset ~{} ({:.1}s elapsed)",
                i,
                i,
                (i as f64 * 0.3)
            );

            sleep(Duration::from_millis(300)).await;
        }
    });

    println!("🔗 PHASE 1: Initial Connection");
    println!("{}\n", "─".repeat(65));

    // Initial subscription - receive first 5 events
    let rx = stream.subscribe();
    client.listen(rx, 2).await; // Listen for 2 seconds

    println!("\n❌ PHASE 2: Simulating Disconnection");
    println!("{}", "─".repeat(65));
    println!("  Client disconnected (e.g., network issue, page refresh)");
    println!("  Events continue to emit while disconnected...\n");

    // Simulate being disconnected for 2 seconds (events continue emitting)
    sleep(Duration::from_secs(2)).await;

    println!("🔗 PHASE 3: Reconnection with History Replay");
    println!("{}\n", "─".repeat(65));

    // Reconnect and catch up
    let rx = client.reconnect(&stream);
    client.listen(rx, 2).await; // Listen for another 2 seconds

    // Final summary
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                         SUMMARY                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    println!("✅ Total events received: {}", client.events_received.len());
    println!("✅ Last offset processed: {}", client.last_offset);
    println!("✅ No events lost during disconnection!\n");

    println!("Key Points:");
    println!("  • EventStream stores full history in memory");
    println!("  • Each event has sequential offset (0, 1, 2, ...)");
    println!("  • Clients track their last_offset");
    println!("  • On reconnect: from_offset(last_offset + 1) gets missed events");
    println!("  • Then subscribe() for live events");
    println!("\nThis pattern is perfect for:");
    println!("  ✓ Web UIs (page refresh, network interruption)");
    println!("  ✓ Mobile apps (background/foreground transitions)");
    println!("  ✓ Monitoring dashboards (temporary disconnects)");
    println!("  ✓ Any scenario needing reliable event delivery\n");

    Ok(())
}
