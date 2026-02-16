# Event Streaming & Observability Guide

**Version:** 0.3.0  
**Last Updated:** 2026-02-16

## Table of Contents
1. [Overview](#overview)
2. [Event System Architecture](#event-system-architecture)
3. [Core Concepts](#core-concepts)
4. [Event Types Reference](#event-types-reference)
5. [Usage Patterns](#usage-patterns)
6. [Advanced Topics](#advanced-topics)
7. [Best Practices](#best-practices)
8. [Examples](#examples)

---

## Overview

The agent-runtime event system provides **complete end-to-end observability** for AI agent workflows. Every component (workflows, agents, LLM requests, tools) emits structured lifecycle events that enable:

- **Real-time monitoring**: Track execution as it happens
- **Streaming responses**: Live LLM token streaming to end users
- **Debugging**: Detailed execution traces with timing
- **Analytics**: Token usage, costs, performance metrics
- **Production observability**: Integration with monitoring systems

### Key Features

✅ **Unified event model**: Consistent `Scope × Type × Status` pattern  
✅ **Component lifecycle tracking**: Started → Running → Completed/Failed  
✅ **Streaming support**: Real-time LLM token streaming  
✅ **Non-blocking**: Async event emission via `tokio::spawn()`  
✅ **Type-safe**: Enforced component ID formats  
✅ **Extensible**: Easy to add new component types

---

## Event System Architecture

### Unified Event Pattern

All events follow the **Scope × Type × Status** pattern:

```
┌─────────────┐   ┌──────────────┐   ┌──────────────────┐
│ EventScope  │ × │  EventType   │ × │ ComponentStatus  │
├─────────────┤   ├──────────────┤   ├──────────────────┤
│ Workflow    │   │ Started      │   │ Pending          │
│ WorkflowStep│   │ Progress     │   │ Running          │
│ Agent       │   │ Completed    │   │ Completed        │
│ LlmRequest  │   │ Failed       │   │ Failed           │
│ Tool        │   │ Canceled     │   │ Canceled         │
│ System      │   │              │   │                  │
└─────────────┘   └──────────────┘   └──────────────────┘
```

### Event Structure

```rust
pub struct Event {
    pub event_id: Uuid,              // Unique event identifier
    pub scope: EventScope,           // Which component type emitted this
    pub event_type: EventType,       // Lifecycle stage (Started, Progress, etc.)
    pub component_id: String,        // Specific component instance
    pub status: ComponentStatus,     // Component's current status
    pub message: Option<String>,     // Human-readable message (errors, progress)
    pub timestamp: DateTime<Utc>,    // When event occurred
    pub workflow_id: Option<String>, // Associated workflow
    pub parent_workflow_id: Option<String>, // For nested workflows
    pub data: JsonValue,             // Component-specific data
}
```

### EventStream

The `EventStream` manages event emission and subscription:

```rust
pub struct EventStream {
    sender: broadcast::Sender<Event>,
    events: Arc<RwLock<Vec<Event>>>,
}

impl EventStream {
    // Subscribe to live events
    pub fn subscribe(&self) -> broadcast::Receiver<Event>
    
    // Get historical events
    pub async fn get_events(&self, offset: usize) -> Vec<Event>
    
    // Emit event (non-blocking)
    pub async fn append(...) -> JoinHandle<Result<Event, String>>
}
```

---

## Core Concepts

### Event Scopes

Six component types emit events:

| Scope | Purpose | Component ID Format | Example |
|-------|---------|---------------------|---------|
| **Workflow** | Overall workflow execution | `workflow_name` | `"data_pipeline"` |
| **WorkflowStep** | Individual workflow steps | `workflow:step:N` | `"pipeline:step:0"` |
| **Agent** | Agent execution | `agent_name` | `"researcher"` |
| **LlmRequest** | LLM API calls | `agent:llm:N` | `"researcher:llm:0"` |
| **Tool** | Tool execution | `tool_name` or `tool:N` | `"calculator"` |
| **System** | Runtime behaviors | `system:subsystem` | `"system:tool_loop"` |

### Event Types (Lifecycle Stages)

All components share the same lifecycle:

```
┌──────────┐
│ Started  │ ──┐
└──────────┘   │
               ├──> ┌──────────┐
┌──────────┐   │    │ Progress │ (optional, repeatable)
│ Pending  │ ──┘    └──────────┘
└──────────┘             │
                         ├──> ┌───────────┐
                         │    │ Completed │
                         │    └───────────┘
                         │
                         ├──> ┌─────────┐
                         │    │ Failed  │
                         │    └─────────┘
                         │
                         └──> ┌───────────┐
                              │ Canceled  │
                              └───────────┘
```

- **Started**: Component begins execution (status: `Running`)
- **Progress**: Component reports intermediate progress (optional, status: `Running`)
- **Completed**: Component finishes successfully (status: `Completed`)
- **Failed**: Component encounters error (status: `Failed`)
- **Canceled**: Component execution canceled (status: `Canceled`)

### Component Status

Tracks component state across events:

- **Pending**: Not yet started
- **Running**: Currently executing
- **Completed**: Finished successfully
- **Failed**: Encountered error
- **Canceled**: Execution canceled

---

## Event Types Reference

### Workflow Events

```rust
use agent_runtime::prelude::*;

// Workflow execution begins
Event::workflow_started(
    "data_pipeline",     // component_id
    Some("wf_123".into()),  // workflow_id
    None,                // parent_workflow_id
    json!({"input": "data.csv"})
);

// Workflow completes
Event::workflow_completed(
    "data_pipeline",
    Some("wf_123".into()),
    None,
    json!({"processed_rows": 1000})
);

// Workflow fails
Event::workflow_failed(
    "data_pipeline",
    "Database connection timeout".to_string(),  // error message
    Some("wf_123".into()),
    None,
    json!({"error_code": "DB_TIMEOUT"})
);
```

**Event Data Fields**:
- `input`: Workflow input (Started)
- `output`: Workflow result (Completed)
- `error`: Error details (Failed)

### WorkflowStep Events

```rust
// Step starts
Event::workflow_step_started(
    "pipeline:step:0",
    Some("wf_123".into()),
    None,
    json!({"step_name": "data_loader", "step_type": "agent"})
);

// Step completes
Event::workflow_step_completed(
    "pipeline:step:0",
    Some("wf_123".into()),
    None,
    json!({
        "step_name": "data_loader",
        "output": {"rows_loaded": 500},
        "duration_ms": 1234
    })
);

// Step fails
Event::workflow_step_failed(
    "pipeline:step:0",
    "Agent execution timeout".to_string(),
    Some("wf_123".into()),
    None,
    json!({"step_name": "data_loader", "timeout_ms": 30000})
);
```

**Event Data Fields**:
- `step_name`: Step identifier
- `step_type`: "agent" | "transform" | "conditional" | "subworkflow"
- `input`: Step input (Started)
- `output`: Step result (Completed)
- `duration_ms`: Execution time (Completed)

### Agent Events

```rust
// Agent starts processing
Event::agent_started(
    "researcher",
    Some("wf_123".into()),
    None,
    json!({"input": "Analyze Q4 sales data"})
);

// Agent reports progress (rare, usually via LlmRequest::Progress)
Event::agent_progress(
    "researcher",
    Some("Processing iteration 2 of 5".into()),
    Some("wf_123".into()),
    None,
    json!({"iteration": 2, "max_iterations": 5})
);

// Agent completes
Event::agent_completed(
    "researcher",
    Some("wf_123".into()),
    None,
    json!({
        "output": "Q4 sales increased 15%...",
        "llm_calls": 3,
        "tool_calls": 5,
        "total_tokens": 1240
    })
);

// Agent fails
Event::agent_failed(
    "researcher",
    "Max iterations exceeded".to_string(),
    Some("wf_123".into()),
    None,
    json!({"iterations": 10, "max_allowed": 10})
);
```

**Event Data Fields**:
- `input`: Agent input (Started)
- `output`: Agent result (Completed)
- `iteration`: Current iteration (Progress)
- `llm_calls`: Number of LLM requests (Completed)
- `tool_calls`: Number of tool executions (Completed)
- `total_tokens`: Cumulative token usage (Completed)

### LlmRequest Events

**These events enable real-time LLM streaming!**

```rust
// LLM request sent
Event::llm_started(
    "researcher:llm:0",
    Some("wf_123".into()),
    None,
    json!({
        "messages": [
            {"role": "system", "content": "You are a researcher..."},
            {"role": "user", "content": "Analyze sales data"}
        ],
        "model": "gpt-4",
        "stream": true
    })
);

// LLM streaming chunks (emitted for each token/batch)
Event::llm_progress(
    "researcher:llm:0",
    Some("Q4".into()),  // The streamed chunk
    Some("wf_123".into()),
    None,
    json!({"chunk": "Q4"})
);

// LLM request completes
Event::llm_completed(
    "researcher:llm:0",
    Some("wf_123".into()),
    None,
    json!({
        "response": "Q4 sales increased 15% compared to Q3...",
        "usage": {
            "prompt_tokens": 120,
            "completion_tokens": 85,
            "total_tokens": 205
        },
        "finish_reason": "stop",
        "tool_calls": []  // If LLM requested tool calls
    })
);

// LLM request fails
Event::llm_failed(
    "researcher:llm:0",
    "API rate limit exceeded".to_string(),
    Some("wf_123".into()),
    None,
    json!({"retry_after": 60, "error_code": "rate_limit_exceeded"})
);
```

**Event Data Fields**:
- `messages`: Chat history sent to LLM (Started)
- `model`: LLM model name (Started)
- `stream`: Whether streaming enabled (Started)
- `chunk`: Token/text chunk (Progress)
- `response`: Complete LLM response (Completed)
- `usage`: Token usage stats (Completed)
- `tool_calls`: Tools requested by LLM (Completed)

### Tool Events

```rust
// Tool execution starts
Event::tool_started(
    "calculator",
    Some("wf_123".into()),
    None,
    json!({
        "tool_name": "calculator",
        "arguments": {"operation": "multiply", "a": 42, "b": 137}
    })
);

// Tool reports progress (for long-running tools)
Event::tool_progress(
    "web_scraper:0",
    Some("Fetched page 3 of 10".into()),
    Some("wf_123".into()),
    None,
    json!({"pages_fetched": 3, "total_pages": 10})
);

// Tool completes
Event::tool_completed(
    "calculator",
    Some("wf_123".into()),
    None,
    json!({
        "tool_name": "calculator",
        "result": 5754,
        "duration_ms": 12
    })
);

// Tool fails
Event::tool_failed(
    "calculator",
    "Division by zero".to_string(),
    Some("wf_123".into()),
    None,
    json!({"tool_name": "calculator", "error_code": "DIV_BY_ZERO"})
);
```

**Event Data Fields**:
- `tool_name`: Tool identifier
- `arguments`: Tool input parameters (Started)
- `result`: Tool output (Completed)
- `duration_ms`: Execution time (Completed)

### System Events

Used for runtime behaviors and diagnostics:

```rust
// Tool loop detection
Event::system_progress(
    "system:tool_loop_detection",
    Some("Tool 'calculator' called with identical arguments".into()),
    Some("wf_123".into()),
    None,
    json!({
        "tool_name": "calculator",
        "arguments": {"a": 5, "b": 3},
        "previous_result": 8,
        "call_count": 2
    })
);

// Custom system events
Event::system_progress(
    "system:rate_limiter",
    Some("Rate limit approaching: 90% of quota used".into()),
    Some("wf_123".into()),
    None,
    json!({"requests_used": 900, "requests_limit": 1000})
);
```

---

## Usage Patterns

### Pattern 1: Basic Event Monitoring

```rust
use agent_runtime::prelude::*;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create event stream
    let (tx, mut rx) = mpsc::channel(100);
    
    // Spawn monitor task
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            println!("[{}] {} - {} ({})",
                event.timestamp.format("%H:%M:%S"),
                event.scope,
                event.event_type,
                event.component_id
            );
        }
    });
    
    // Run workflow with events
    let workflow = Workflow::new("example")
        .add_step(AgentStep::new(agent))
        .build();
    
    workflow.execute(input, &mut rx).await?;
    Ok(())
}
```

### Pattern 2: Filtering Specific Events

```rust
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        match (event.scope, event.event_type) {
            // Track all failures
            (_, EventType::Failed) => {
                eprintln!("❌ {} failed: {}",
                    event.component_id,
                    event.message.unwrap_or_default()
                );
            }
            
            // Track LLM streaming
            (EventScope::LlmRequest, EventType::Progress) => {
                if let Some(chunk) = event.data["chunk"].as_str() {
                    print!("{}", chunk);
                    io::stdout().flush().unwrap();
                }
            }
            
            // Track tool executions
            (EventScope::Tool, EventType::Completed) => {
                let result = &event.data["result"];
                println!("🔧 {} → {}", event.component_id, result);
            }
            
            _ => {}
        }
    }
});
```

### Pattern 3: Real-time LLM Streaming

```rust
// Stream LLM responses to end user
tokio::spawn(async move {
    let mut current_response = String::new();
    
    while let Some(event) = rx.recv().await {
        match (event.scope, event.event_type) {
            (EventScope::LlmRequest, EventType::Started) => {
                println!("\n🤖 Assistant is thinking...\n");
                current_response.clear();
            }
            
            (EventScope::LlmRequest, EventType::Progress) => {
                if let Some(chunk) = event.data["chunk"].as_str() {
                    print!("{}", chunk);
                    io::stdout().flush().unwrap();
                    current_response.push_str(chunk);
                }
            }
            
            (EventScope::LlmRequest, EventType::Completed) => {
                println!("\n\n✅ Complete ({} tokens)",
                    event.data["usage"]["total_tokens"]
                );
            }
            
            _ => {}
        }
    }
});
```

### Pattern 4: Component Status Tracking

```rust
use std::collections::HashMap;

let mut statuses: HashMap<String, ComponentStatus> = HashMap::new();
let mut timings: HashMap<String, Instant> = HashMap::new();

while let Some(event) = rx.recv().await {
    // Update status
    statuses.insert(event.component_id.clone(), event.status);
    
    // Track timing
    match event.event_type {
        EventType::Started => {
            timings.insert(event.component_id.clone(), Instant::now());
        }
        EventType::Completed | EventType::Failed => {
            if let Some(start) = timings.remove(&event.component_id) {
                let duration = start.elapsed();
                println!("{} took {:?}", event.component_id, duration);
            }
        }
        _ => {}
    }
    
    // Show progress
    let running: Vec<_> = statuses.iter()
        .filter(|(_, s)| **s == ComponentStatus::Running)
        .map(|(id, _)| id)
        .collect();
    
    if !running.is_empty() {
        println!("Running: {:?}", running);
    }
}
```

### Pattern 5: Token Usage Tracking

```rust
let mut total_tokens = 0;
let mut total_cost = 0.0;

// Pricing: $0.03 per 1K input tokens, $0.06 per 1K output tokens
const INPUT_COST: f64 = 0.03 / 1000.0;
const OUTPUT_COST: f64 = 0.06 / 1000.0;

while let Some(event) = rx.recv().await {
    if let (EventScope::LlmRequest, EventType::Completed) = (event.scope, event.event_type) {
        let usage = &event.data["usage"];
        let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0);
        let completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0);
        
        total_tokens += prompt_tokens + completion_tokens;
        total_cost += (prompt_tokens as f64 * INPUT_COST) 
                    + (completion_tokens as f64 * OUTPUT_COST);
        
        println!("💰 Request cost: ${:.4} ({} tokens)",
            (prompt_tokens as f64 * INPUT_COST) + (completion_tokens as f64 * OUTPUT_COST),
            prompt_tokens + completion_tokens
        );
    }
}

println!("\n📊 Total: {} tokens, ${:.2}", total_tokens, total_cost);
```

### Pattern 6: Error Handling & Retries

```rust
let mut retry_queue: Vec<Event> = Vec::new();

while let Some(event) = rx.recv().await {
    match (event.scope, event.event_type) {
        (EventScope::LlmRequest, EventType::Failed) => {
            let error = event.message.as_ref().unwrap();
            
            if error.contains("rate_limit") {
                println!("⏸  Rate limited, will retry in 60s");
                retry_queue.push(event);
            } else if error.contains("timeout") {
                println!("⚠  Timeout, retrying with longer timeout");
                retry_queue.push(event);
            } else {
                eprintln!("❌ Permanent failure: {}", error);
            }
        }
        
        (EventScope::Tool, EventType::Failed) => {
            println!("🔧 Tool {} failed: {}",
                event.component_id,
                event.message.unwrap_or_default()
            );
            // Tools usually shouldn't be retried
        }
        
        _ => {}
    }
}
```

---

## Advanced Topics

### Event Stream Replay

Retrieve historical events for replay/debugging:

```rust
let runtime = Runtime::new();

// Get all events from start
let all_events = runtime.event_stream().get_events(0).await;

// Get events from offset (for reconnection)
let recent_events = runtime.event_stream().get_events(100).await;

// Replay events
for event in all_events {
    println!("{:?}", event);
}
```

### Multi-Subscriber Pattern

Multiple components can subscribe to the same event stream:

```rust
let runtime = Runtime::new();

// Subscriber 1: UI updates
let mut ui_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = ui_rx.recv().await {
        update_ui(event);
    }
});

// Subscriber 2: Logging
let mut log_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = log_rx.recv().await {
        log_event(event);
    }
});

// Subscriber 3: Metrics
let mut metrics_rx = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = metrics_rx.recv().await {
        record_metrics(event);
    }
});
```

### Custom Event Data

Add custom fields to event data:

```rust
Event::agent_completed(
    "researcher",
    Some("wf_123".into()),
    None,
    json!({
        // Standard fields
        "output": "Analysis complete",
        "total_tokens": 500,
        
        // Custom fields
        "custom_metric": 42,
        "tags": ["production", "high-priority"],
        "user_id": "user_123",
        "request_id": "req_abc"
    })
);
```

### Nested Workflows

Track parent-child workflow relationships:

```rust
// Parent workflow
Event::workflow_started(
    "parent_workflow",
    Some("wf_parent".into()),
    None,  // no parent
    json!({})
);

// Child workflow (subworkflow)
Event::workflow_started(
    "child_workflow",
    Some("wf_child".into()),
    Some("wf_parent".into()),  // parent_workflow_id
    json!({"parent": "wf_parent"})
);

// Filter by parent
while let Some(event) = rx.recv().await {
    if event.parent_workflow_id == Some("wf_parent".to_string()) {
        println!("Child workflow event: {:?}", event);
    }
}
```

---

## Best Practices

### 1. Event Monitoring

✅ **DO**: Use separate async tasks for event monitoring  
✅ **DO**: Handle `Err` from broadcast receiver (indicates lag/overflow)  
✅ **DO**: Use pattern matching on `(scope, type)` tuples  
❌ **DON'T**: Block agent execution waiting for events  
❌ **DON'T**: Store unbounded event history in memory

```rust
// ✅ Good: Separate task
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // Process event
    }
});

// ❌ Bad: Blocking main thread
for event in rx.recv().await {
    // Blocks workflow execution
}
```

### 2. Error Handling

✅ **DO**: Always check `message` field for Failed events  
✅ **DO**: Log error details from `event.data`  
✅ **DO**: Track error rates and patterns  
❌ **DON'T**: Ignore Failed events silently

```rust
// ✅ Good
match (event.scope, event.event_type) {
    (_, EventType::Failed) => {
        let error_msg = event.message.unwrap_or_else(|| "Unknown error".into());
        let error_data = &event.data;
        eprintln!("[{}] {} failed: {}\nData: {}",
            event.component_id, event.scope, error_msg, error_data);
    }
    _ => {}
}
```

### 3. Streaming Performance

✅ **DO**: Batch LLM chunks for UI updates (Future: automatic batching in v0.4.0)  
✅ **DO**: Use buffered channels (e.g., `mpsc::channel(100)`)  
✅ **DO**: Flush output after streaming chunks  
❌ **DON'T**: Send each token individually to network (causes flicker)

```rust
// ✅ Good: Batch chunks (manual for now)
let mut buffer = String::new();
let mut last_flush = Instant::now();

while let Some(event) = rx.recv().await {
    if let (EventScope::LlmRequest, EventType::Progress) = (event.scope, event.event_type) {
        buffer.push_str(event.data["chunk"].as_str().unwrap());
        
        // Flush every 50ms
        if last_flush.elapsed() > Duration::from_millis(50) {
            print!("{}", buffer);
            io::stdout().flush().unwrap();
            buffer.clear();
            last_flush = Instant::now();
        }
    }
}
```

### 4. Component IDs

✅ **DO**: Follow enforced formats for each scope  
✅ **DO**: Use descriptive, unique component IDs  
✅ **DO**: Include context in IDs (e.g., `agent:llm:0` not just `llm`)  
❌ **DON'T**: Use empty or generic IDs like `"agent"` for everything

```rust
// ✅ Good
Event::tool_started("web_scraper:batch_1", ...);
Event::llm_started("researcher:llm:2", ...);

// ❌ Bad
Event::tool_started("tool", ...);  // Not descriptive
Event::llm_started("llm", ...);    // Missing agent context
```

### 5. Message Fields

✅ **DO**: Provide human-readable messages for Progress/Failed events  
✅ **DO**: Keep messages concise but informative  
❌ **DON'T**: Put large data in message field (use `data` instead)

```rust
// ✅ Good
Event::tool_progress(
    "data_loader",
    Some("Loaded 500/1000 rows".into()),
    ...
);

// ❌ Bad
Event::tool_progress(
    "data_loader",
    Some(format!("{:?}", all_loaded_data)),  // Too large!
    ...
);
```

---

## Examples

### Example 1: Complete Workflow Monitor

```rust
use agent_runtime::prelude::*;
use std::io::{self, Write};
use std::time::Instant;

async fn monitor_workflow() {
    let (tx, mut rx) = mpsc::channel(100);
    let start_time = Instant::now();
    
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let elapsed = start_time.elapsed().as_secs_f64();
            
            match (event.scope, event.event_type) {
                (EventScope::Workflow, EventType::Started) => {
                    println!("\n🚀 [{:.2}s] Workflow '{}' started",
                        elapsed, event.component_id);
                }
                
                (EventScope::WorkflowStep, EventType::Started) => {
                    println!("  ▶ [{:.2}s] Step '{}'",
                        elapsed, event.component_id);
                }
                
                (EventScope::Agent, EventType::Started) => {
                    println!("    🤖 [{:.2}s] Agent '{}' processing",
                        elapsed, event.component_id);
                }
                
                (EventScope::LlmRequest, EventType::Started) => {
                    println!("      💬 [{:.2}s] LLM request", elapsed);
                }
                
                (EventScope::LlmRequest, EventType::Progress) => {
                    if let Some(chunk) = event.data["chunk"].as_str() {
                        print!("{}", chunk);
                        io::stdout().flush().unwrap();
                    }
                }
                
                (EventScope::LlmRequest, EventType::Completed) => {
                    let tokens = event.data["usage"]["total_tokens"]
                        .as_u64().unwrap_or(0);
                    println!("\n      ✅ [{:.2}s] LLM complete ({} tokens)",
                        elapsed, tokens);
                }
                
                (EventScope::Tool, EventType::Started) => {
                    let args = &event.data["arguments"];
                    println!("      🔧 [{:.2}s] Tool '{}' args: {}",
                        elapsed, event.component_id, args);
                }
                
                (EventScope::Tool, EventType::Completed) => {
                    let result = &event.data["result"];
                    println!("      ✓ [{:.2}s] Tool result: {}",
                        elapsed, result);
                }
                
                (EventScope::WorkflowStep, EventType::Completed) => {
                    let duration = event.data["duration_ms"]
                        .as_u64().unwrap_or(0);
                    println!("  ✅ [{:.2}s] Step complete ({}ms)",
                        elapsed, duration);
                }
                
                (EventScope::Workflow, EventType::Completed) => {
                    println!("\n✅ [{:.2}s] Workflow complete\n", elapsed);
                }
                
                (_, EventType::Failed) => {
                    let msg = event.message.unwrap_or_default();
                    eprintln!("\n❌ [{:.2}s] {} failed: {}",
                        elapsed, event.component_id, msg);
                }
                
                _ => {}
            }
        }
    });
    
    // Run workflow...
}
```

### Example 2: Production Metrics Collection

```rust
use std::collections::HashMap;

struct Metrics {
    llm_calls: usize,
    total_tokens: u64,
    tool_calls: HashMap<String, usize>,
    errors: Vec<String>,
    execution_times: HashMap<String, u128>,
}

async fn collect_metrics(mut rx: mpsc::Receiver<Event>) -> Metrics {
    let mut metrics = Metrics {
        llm_calls: 0,
        total_tokens: 0,
        tool_calls: HashMap::new(),
        errors: Vec::new(),
        execution_times: HashMap::new(),
    };
    
    let mut start_times: HashMap<String, Instant> = HashMap::new();
    
    while let Some(event) = rx.recv().await {
        match (event.scope, event.event_type) {
            (EventScope::LlmRequest, EventType::Completed) => {
                metrics.llm_calls += 1;
                if let Some(tokens) = event.data["usage"]["total_tokens"].as_u64() {
                    metrics.total_tokens += tokens;
                }
            }
            
            (EventScope::Tool, EventType::Started) => {
                let tool = event.component_id.clone();
                *metrics.tool_calls.entry(tool.clone()).or_insert(0) += 1;
                start_times.insert(tool, Instant::now());
            }
            
            (EventScope::Tool, EventType::Completed) => {
                if let Some(start) = start_times.remove(&event.component_id) {
                    let duration = start.elapsed().as_millis();
                    metrics.execution_times.insert(event.component_id.clone(), duration);
                }
            }
            
            (_, EventType::Failed) => {
                let error = format!("{}: {}",
                    event.component_id,
                    event.message.unwrap_or_default()
                );
                metrics.errors.push(error);
            }
            
            _ => {}
        }
    }
    
    metrics
}
```

### Example 3: WebSocket Event Streaming

```rust
// For actix-web or similar
use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;

async fn ws_events(
    req: HttpRequest,
    stream: web::Payload,
    runtime: web::Data<Runtime>,
) -> Result<HttpResponse, Error> {
    let mut rx = runtime.event_stream().subscribe();
    
    ws::start(
        EventWebSocket { rx },
        &req,
        stream,
    )
}

struct EventWebSocket {
    rx: broadcast::Receiver<Event>,
}

impl Actor for EventWebSocket {
    type Context = ws::WebsocketContext<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        let rx = self.rx.resubscribe();
        
        ctx.spawn(
            async move {
                while let Ok(event) = rx.recv().await {
                    // Send event to client
                    let json = serde_json::to_string(&event).unwrap();
                    ctx.text(json);
                }
            }
            .into_actor(self),
        );
    }
}
```

---

## Future Enhancements (v0.4.0+)

### Automatic Streaming Batching

Future versions will automatically batch high-frequency events:

- **LLM chunks**: Batched over 50ms windows
- **Tool progress**: Batched over 100ms windows
- Reduces network overhead for real-time UIs

### Tool Progress Callbacks

Tools will support streaming progress:

```rust
// Future API
#[async_trait]
impl Tool for LongRunningTool {
    async fn execute_with_progress(
        &self,
        args: JsonValue,
        progress: ProgressCallback,
    ) -> Result<JsonValue, String> {
        for i in 0..100 {
            process_item(i);
            progress(format!("Processed {}/100", i), Some(i));
        }
        Ok(json!({"status": "done"}))
    }
}
```

### Event Persistence

Planned support for persisting events to storage:

- Database integration (Postgres, SQLite)
- S3/object storage for archival
- Event replay from storage

### Advanced Filtering

Subscription-time event filtering:

```rust
// Future API
runtime.event_stream()
    .subscribe()
    .filter(EventScope::LlmRequest)
    .filter_status(ComponentStatus::Failed)
```

---

## See Also

- [MIGRATION_0.2_TO_0.3.md](MIGRATION_0.2_TO_0.3.md) - Upgrade guide from v0.2.x
- [CHANGELOG.md](../CHANGELOG.md) - Release notes for v0.3.0
- [docs/vnext/ASYNC_EVENTS.md](vnext/ASYNC_EVENTS.md) - Future async architecture
- Examples: `src/bin/workflow_demo.rs`, `src/bin/multi_subscriber.rs`
