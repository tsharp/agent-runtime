# Building UIs with the Async Event System

The v0.3.0 unified event system is designed specifically to support real-time UI updates. This guide shows how to integrate various UI frameworks.

## Core Concept

The `EventStream` uses Tokio's `broadcast` channel, which supports **multiple subscribers**. Any UI can subscribe and receive real-time events as workflows execute.

```rust
use agent_runtime::{Runtime, Event};

// Create runtime
let runtime = Runtime::new();

// Subscribe to events (can have multiple subscribers)
let mut ui_rx = runtime.event_stream().subscribe();

// Listen for events in UI thread/task
tokio::spawn(async move {
    while let Ok(event) = ui_rx.recv().await {
        // Update UI with event
        update_ui(event);
    }
});
```

## Architecture Patterns

### Pattern 1: WebSocket Server (Web UI)

Stream events to web browsers via WebSocket:

```rust
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};
use agent_runtime::Runtime;
use std::sync::Arc;

async fn ws_events(
    ws: WebSocketUpgrade,
    runtime: Arc<Runtime>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, runtime))
}

async fn handle_socket(mut socket: WebSocket, runtime: Arc<Runtime>) {
    let mut rx = runtime.event_stream().subscribe();
    
    while let Ok(event) = rx.recv().await {
        // Serialize event to JSON and send to browser
        let json = serde_json::to_string(&event).unwrap();
        if socket.send(Message::Text(json)).await.is_err() {
            break; // Client disconnected
        }
    }
}

#[tokio::main]
async fn main() {
    let runtime = Arc::new(Runtime::new());
    
    let app = Router::new()
        .route("/events", get(ws_events))
        .with_state(runtime.clone());
    
    // Start server
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Pattern 2: Server-Sent Events (SSE)

For one-way streaming to browsers:

```rust
use axum::{
    response::sse::{Event as SseEvent, Sse},
    routing::get,
    Router,
};
use futures::stream::{Stream, StreamExt};
use agent_runtime::Runtime;
use std::sync::Arc;

async fn sse_events(
    runtime: Arc<Runtime>,
) -> Sse<impl Stream<Item = Result<SseEvent, std::convert::Infallible>>> {
    let rx = runtime.event_stream().subscribe();
    
    let stream = async_stream::stream! {
        let mut rx = rx;
        while let Ok(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap();
            yield Ok(SseEvent::default().data(json));
        }
    };
    
    Sse::new(stream)
}

#[tokio::main]
async fn main() {
    let runtime = Arc::new(Runtime::new());
    
    let app = Router::new()
        .route("/events", get(sse_events))
        .with_state(runtime);
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Pattern 3: Desktop UI (egui, iced, etc.)

For native desktop applications:

```rust
use agent_runtime::{Runtime, Event, EventScope, EventType};
use tokio::sync::mpsc;
use std::sync::Arc;

struct AppState {
    runtime: Arc<Runtime>,
    events: Vec<Event>,
    current_status: String,
}

impl AppState {
    fn new() -> Self {
        let runtime = Arc::new(Runtime::new());
        
        // Spawn event listener
        let runtime_clone = runtime.clone();
        let (tx, mut rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            let mut event_rx = runtime_clone.event_stream().subscribe();
            while let Ok(event) = event_rx.recv().await {
                let _ = tx.send(event).await;
            }
        });
        
        Self {
            runtime,
            events: Vec::new(),
            current_status: "Ready".to_string(),
        }
    }
    
    fn update(&mut self, event: Event) {
        // Update UI state based on event
        match (event.scope, event.event_type) {
            (EventScope::Workflow, EventType::Started) => {
                self.current_status = format!("Workflow {} started", event.component_id);
            }
            (EventScope::Agent, EventType::Started) => {
                self.current_status = format!("Agent {} processing", event.component_id);
            }
            (EventScope::LlmRequest, EventType::Progress) => {
                // Streaming LLM response
                if let Some(chunk) = event.data.get("chunk") {
                    self.current_status.push_str(chunk.as_str().unwrap_or(""));
                }
            }
            _ => {}
        }
        
        self.events.push(event);
    }
}
```

### Pattern 4: Terminal UI (ratatui)

For rich terminal interfaces:

```rust
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use agent_runtime::{Event, Runtime, EventScope, EventType};
use std::sync::Arc;
use tokio::sync::mpsc;

async fn run_tui(runtime: Arc<Runtime>) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    
    let (event_tx, mut event_rx) = mpsc::channel(100);
    
    // Spawn event listener
    let runtime_clone = runtime.clone();
    tokio::spawn(async move {
        let mut rx = runtime_clone.event_stream().subscribe();
        while let Ok(event) = rx.recv().await {
            let _ = event_tx.send(event).await;
        }
    });
    
    let mut events = Vec::new();
    
    loop {
        // Try to receive new events
        while let Ok(event) = event_rx.try_recv() {
            events.push(format!(
                "[{:?}] {:?}::{}",
                event.timestamp.format("%H:%M:%S"),
                event.scope,
                event.event_type
            ));
        }
        
        // Render UI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(f.size());
            
            let items: Vec<ListItem> = events.iter().map(|e| ListItem::new(e.as_str())).collect();
            let list = List::new(items).block(Block::default().title("Events").borders(Borders::ALL));
            
            f.render_widget(list, chunks[0]);
        })?;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
```

## Real-World Example: React Frontend

### Backend (Axum + WebSocket)

```rust
// src/main.rs
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade, State},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use agent_runtime::{Runtime, Workflow, WorkflowBuilder};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    runtime: Arc<Runtime>,
    workflows: Arc<RwLock<Vec<Workflow>>>,
}

// WebSocket endpoint for event streaming
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

async fn handle_ws_connection(mut socket: WebSocket, state: AppState) {
    let mut rx = state.runtime.event_stream().subscribe();
    
    while let Ok(event) = rx.recv().await {
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(_) => continue,
        };
        
        if socket.send(axum::extract::ws::Message::Text(json)).await.is_err() {
            break;
        }
    }
}

// REST endpoint to start workflow
async fn start_workflow(
    State(state): State<AppState>,
    Json(input): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Start workflow execution (returns immediately, events stream via WebSocket)
    let workflow = state.workflows.read().await[0].clone();
    
    tokio::spawn(async move {
        let _ = state.runtime.execute(workflow).await;
    });
    
    Json(serde_json::json!({"status": "started"}))
}

#[tokio::main]
async fn main() {
    let runtime = Arc::new(Runtime::new());
    
    let state = AppState {
        runtime,
        workflows: Arc::new(RwLock::new(Vec::new())),
    };
    
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/workflows/start", post(start_workflow))
        .with_state(state);
    
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

### Frontend (React + TypeScript)

```typescript
// EventStream.tsx
import { useEffect, useState } from 'react';

interface Event {
  event_id: string;
  scope: 'Workflow' | 'WorkflowStep' | 'Agent' | 'LlmRequest' | 'Tool' | 'System';
  event_type: 'Started' | 'Progress' | 'Completed' | 'Failed' | 'Canceled';
  component_id: string;
  status: 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Canceled';
  message?: string;
  timestamp: string;
  data: any;
}

export const useEventStream = () => {
  const [events, setEvents] = useState<Event[]>([]);
  const [status, setStatus] = useState<'connecting' | 'connected' | 'disconnected'>('disconnected');

  useEffect(() => {
    const ws = new WebSocket('ws://localhost:3000/ws');
    
    ws.onopen = () => setStatus('connected');
    ws.onclose = () => setStatus('disconnected');
    
    ws.onmessage = (msg) => {
      const event: Event = JSON.parse(msg.data);
      setEvents(prev => [...prev, event]);
    };
    
    return () => ws.close();
  }, []);

  return { events, status };
};

// WorkflowMonitor.tsx
export const WorkflowMonitor = () => {
  const { events, status } = useEventStream();
  const [llmResponse, setLlmResponse] = useState('');

  useEffect(() => {
    // Update LLM response from streaming chunks
    const latestEvent = events[events.length - 1];
    if (latestEvent?.scope === 'LlmRequest' && latestEvent?.event_type === 'Progress') {
      const chunk = latestEvent.data.chunk || '';
      setLlmResponse(prev => prev + chunk);
    }
  }, [events]);

  return (
    <div className="workflow-monitor">
      <div className="status">Connection: {status}</div>
      
      <div className="llm-response">
        <h3>LLM Response</h3>
        <p>{llmResponse}</p>
      </div>
      
      <div className="event-log">
        <h3>Event Log</h3>
        {events.map(event => (
          <div key={event.event_id} className={`event ${event.event_type.toLowerCase()}`}>
            <span className="scope">{event.scope}</span>
            <span className="type">{event.event_type}</span>
            <span className="component">{event.component_id}</span>
            {event.message && <span className="message">{event.message}</span>}
          </div>
        ))}
      </div>
    </div>
  );
};
```

## Multi-Subscriber Pattern

The event system supports **unlimited subscribers**, each receiving all events:

```rust
let runtime = Runtime::new();

// UI subscriber
let mut ui_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = ui_rx.recv().await {
        update_ui(event);
    }
});

// Logging subscriber
let mut log_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = log_rx.recv().await {
        log_to_file(event);
    }
});

// Metrics subscriber
let mut metrics_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = metrics_rx.recv().await {
        record_metrics(event);
    }
});

// Database subscriber
let mut db_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = db_rx.recv().await {
        save_to_database(event);
    }
});
```

## Event Filtering in UI

Filter events by scope, type, or component:

```rust
while let Ok(event) = rx.recv().await {
    match (event.scope, event.event_type) {
        // Only show workflow-level events in top status bar
        (EventScope::Workflow, _) => {
            update_status_bar(event);
        }
        
        // Show LLM streaming in response area
        (EventScope::LlmRequest, EventType::Progress) => {
            append_to_response_area(event.data["chunk"]);
        }
        
        // Show all failures in error panel
        (_, EventType::Failed) => {
            show_error_notification(event);
        }
        
        // Log everything to debug panel
        _ => {
            append_to_debug_log(event);
        }
    }
}
```

## Performance Considerations

### Buffering High-Frequency Events

For UIs, you may want to batch rapid events:

```rust
use tokio::time::{interval, Duration};

let mut rx = runtime.event_stream().subscribe();
let mut buffer = Vec::new();
let mut tick = interval(Duration::from_millis(100));

loop {
    tokio::select! {
        Ok(event) = rx.recv() => {
            buffer.push(event);
        }
        _ = tick.tick() => {
            if !buffer.is_empty() {
                update_ui_batch(&buffer);
                buffer.clear();
            }
        }
    }
}
```

### Limiting Event History

Prevent memory growth in long-running UIs:

```rust
const MAX_EVENTS: usize = 1000;

let mut events = Vec::new();

while let Ok(event) = rx.recv().await {
    events.push(event);
    
    // Keep only recent events
    if events.len() > MAX_EVENTS {
        events.drain(0..events.len() - MAX_EVENTS);
    }
    
    update_ui(&events);
}
```

## Complete Example: Progress Bar UI

```rust
use indicatif::{ProgressBar, ProgressStyle};
use agent_runtime::{Runtime, Event, EventScope, EventType};
use std::sync::Arc;

async fn run_workflow_with_progress(runtime: Arc<Runtime>) {
    let mut rx = runtime.event_stream().subscribe();
    
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
    );
    
    let mut total_steps = 0;
    let mut completed_steps = 0;
    
    while let Ok(event) = rx.recv().await {
        match (event.scope, event.event_type) {
            (EventScope::Workflow, EventType::Started) => {
                if let Some(steps) = event.data.get("num_steps") {
                    total_steps = steps.as_u64().unwrap_or(0);
                    pb.set_length(total_steps);
                }
                pb.set_message("Workflow started");
            }
            
            (EventScope::WorkflowStep, EventType::Completed) => {
                completed_steps += 1;
                pb.set_position(completed_steps);
                pb.set_message(format!("Step {} complete", event.component_id));
            }
            
            (EventScope::LlmRequest, EventType::Progress) => {
                if let Some(chunk) = event.data.get("chunk") {
                    pb.println(chunk.as_str().unwrap_or(""));
                }
            }
            
            (EventScope::Workflow, EventType::Completed) => {
                pb.finish_with_message("✓ Workflow complete");
                break;
            }
            
            (_, EventType::Failed) => {
                pb.abandon_with_message(format!("✗ Failed: {}", event.message.unwrap_or_default()));
                break;
            }
            
            _ => {}
        }
    }
}
```

## Benefits for UI Development

### 1. Real-Time Updates
- No polling required
- Events arrive immediately as they happen
- Perfect for progress indicators, live logs, streaming responses

### 2. Separation of Concerns
- Backend focuses on workflow execution
- Frontend focuses on presentation
- Events provide clean interface between layers

### 3. Multiple Views
- Same event stream can power multiple UI components
- Dashboard, debug panel, notifications all from one source

### 4. Historical Replay
- `event_stream().get_events(offset)` allows UI reconnection without losing history
- Perfect for page refreshes or mobile app backgrounds

### 5. Testable
- Easy to mock event streams for UI testing
- Replay recorded events for UI development

## See Also

- [EVENT_STREAMING.md](EVENT_STREAMING.md) - Complete event system guide
- [MIGRATION_0.2_TO_0.3.md](MIGRATION_0.2_TO_0.3.md) - Migration from v0.2.x
- Example: `src/bin/async_events_demo.rs` - Basic event monitoring
- Example: `src/bin/workflow_demo.rs` - Real workflow with event display
