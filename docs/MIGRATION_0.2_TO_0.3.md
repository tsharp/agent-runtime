# Migration Guide: v0.2.x → v0.3.0

This guide helps you migrate from agent-runtime v0.2.x to v0.3.0, which introduces a **unified event system** with breaking changes to the event API.

## Overview of Changes

### What Changed
- **Event structure redesigned**: Introduced `EventScope`, `EventType`, and `ComponentStatus` for unified lifecycle tracking
- **19 old event types replaced**: All old specific event types (e.g., `AgentLlmRequestStarted`, `WorkflowStepCompleted`) consolidated into 5 lifecycle events
- **Component ID format enforcement**: Standardized component identification across all event scopes
- **Helper methods added**: Ergonomic API for common event patterns
- **Pattern matching updates**: Event handling now uses `(scope, type)` tuple matching

### Why This Change?
The new unified event system provides:
- **Consistency**: All components (workflows, agents, tools) share the same lifecycle events
- **Extensibility**: Easy to add new component types without proliferating event variants
- **Observability**: Complete visibility into execution with predictable event patterns
- **Type safety**: Structured component IDs with validation

---

## Breaking Changes

### 1. Event Structure

#### Before (v0.2.x)
```rust
pub struct Event {
    pub event_id: Uuid,
    pub event_type: EventType,  // 19 different variants
    pub timestamp: DateTime<Utc>,
    pub agent_name: Option<String>,
    pub workflow_id: Option<String>,
    pub parent_workflow_id: Option<String>,
    pub data: JsonValue,
}
```

#### After (v0.3.0)
```rust
pub struct Event {
    pub event_id: Uuid,
    pub scope: EventScope,          // NEW: Which component emitted
    pub event_type: EventType,      // CHANGED: Only 5 lifecycle types
    pub component_id: String,       // NEW: Identifies specific component
    pub status: ComponentStatus,    // NEW: Component's current status
    pub message: Option<String>,    // NEW: Optional human-readable message
    pub timestamp: DateTime<Utc>,
    pub workflow_id: Option<String>,
    pub parent_workflow_id: Option<String>,
    pub data: JsonValue,
}
```

**Migration Action**: Update all code that accesses event fields.

---

### 2. EventType Enum

#### Before (v0.2.x)
```rust
pub enum EventType {
    // Workflow events (6)
    WorkflowStarted,
    WorkflowStepStarted,
    WorkflowStepCompleted,
    WorkflowStepFailed,
    WorkflowCompleted,
    WorkflowFailed,
    
    // Agent events (7)
    AgentProcessing,
    AgentCompleted,
    AgentFailed,
    AgentLlmRequestStarted,
    AgentLlmRequestCompleted,
    AgentLlmRequestFailed,
    AgentLlmStreamChunk,
    
    // Tool events (4)
    ToolCallStarted,
    ToolCallCompleted,
    ToolCallFailed,
    AgentToolLoopDetected,
    
    // Other (2)
    TransformStepCompleted,
    CustomEvent,
}
```

#### After (v0.3.0)
```rust
pub enum EventScope {
    Workflow,
    WorkflowStep,
    Agent,
    LlmRequest,
    Tool,
    System,
}

pub enum EventType {
    Started,
    Progress,
    Completed,
    Failed,
    Canceled,
}

pub enum ComponentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
}
```

**Migration Action**: Replace specific event types with `(scope, type)` combinations.

---

### 3. Event Matching Patterns

#### Before (v0.2.x)
```rust
match event.event_type {
    EventType::WorkflowStarted => {
        println!("Workflow started: {}", event.workflow_id.unwrap());
    }
    EventType::AgentLlmStreamChunk => {
        let chunk = event.data.get("chunk").unwrap();
        print!("{}", chunk);
    }
    EventType::ToolCallCompleted => {
        println!("Tool completed: {}", event.data["tool_name"]);
    }
    _ => {}
}
```

#### After (v0.3.0)
```rust
match (event.scope, event.event_type) {
    (EventScope::Workflow, EventType::Started) => {
        println!("Workflow started: {} ({})", 
            event.component_id, event.workflow_id.unwrap());
    }
    (EventScope::LlmRequest, EventType::Progress) => {
        let chunk = event.data.get("chunk").unwrap();
        print!("{}", chunk);
    }
    (EventScope::Tool, EventType::Completed) => {
        println!("Tool completed: {}", event.component_id);
    }
    _ => {}
}
```

**Migration Action**: Update all event pattern matching to use `(scope, type)` tuples.

---

### 4. Event Creation

#### Before (v0.2.x)
```rust
// Manual event creation (rare)
let event = Event {
    event_id: Uuid::new_v4(),
    event_type: EventType::ToolCallStarted,
    timestamp: Utc::now(),
    agent_name: Some("my_agent".to_string()),
    workflow_id: None,
    parent_workflow_id: None,
    data: json!({
        "tool_name": "calculator",
        "arguments": {"a": 5, "b": 3}
    }),
};
```

#### After (v0.3.0)
```rust
// Use helper methods (recommended)
let event = Event::tool_started(
    "calculator",           // component_id
    None,                   // workflow_id
    None,                   // parent_workflow_id
    json!({"a": 5, "b": 3}) // data
);

// Or raw construction (advanced)
let event = Event::new(
    EventScope::Tool,
    EventType::Started,
    "calculator",
    ComponentStatus::Running,
    None,  // message
    None,  // workflow_id
    None,  // parent_workflow_id
    json!({"a": 5, "b": 3}),
);
```

**Migration Action**: Use helper methods for event creation.

---

### 5. EventStream::append() Signature

#### Before (v0.2.x)
```rust
pub async fn append(
    &self,
    event_type: EventType,
    workflow_id: Option<String>,
    data: JsonValue,
) -> Result<Event, String>
```

#### After (v0.3.0)
```rust
pub async fn append(
    &self,
    scope: EventScope,
    event_type: EventType,
    component_id: String,
    status: ComponentStatus,
    message: Option<String>,
    workflow_id: Option<String>,
    parent_workflow_id: Option<String>,
    data: JsonValue,
) -> JoinHandle<Result<Event, String>>
```

**Migration Actions**:
1. Add new required parameters (`scope`, `component_id`, `status`, `message`, `parent_workflow_id`)
2. Handle returned `JoinHandle` (or ignore if fire-and-forget)
3. **Recommended**: Use helper methods instead of raw `append()`

---

## Event Type Migration Table

| Old Event (v0.2.x) | New Pattern (v0.3.0) | Helper Method |
|--------------------|----------------------|---------------|
| `WorkflowStarted` | `(Workflow, Started)` | `Event::workflow_started()` |
| `WorkflowStepStarted` | `(WorkflowStep, Started)` | `Event::workflow_step_started()` |
| `WorkflowStepCompleted` | `(WorkflowStep, Completed)` | `Event::workflow_step_completed()` |
| `WorkflowStepFailed` | `(WorkflowStep, Failed)` | `Event::workflow_step_failed()` |
| `WorkflowCompleted` | `(Workflow, Completed)` | `Event::workflow_completed()` |
| `WorkflowFailed` | `(Workflow, Failed)` | `Event::workflow_failed()` |
| `AgentProcessing` | `(Agent, Started)` | `Event::agent_started()` |
| `AgentCompleted` | `(Agent, Completed)` | `Event::agent_completed()` |
| `AgentFailed` | `(Agent, Failed)` | `Event::agent_failed()` |
| `AgentLlmRequestStarted` | `(LlmRequest, Started)` | `Event::llm_started()` |
| `AgentLlmStreamChunk` | `(LlmRequest, Progress)` | `Event::llm_progress()` |
| `AgentLlmRequestCompleted` | `(LlmRequest, Completed)` | `Event::llm_completed()` |
| `AgentLlmRequestFailed` | `(LlmRequest, Failed)` | `Event::llm_failed()` |
| `ToolCallStarted` | `(Tool, Started)` | `Event::tool_started()` |
| `ToolCallCompleted` | `(Tool, Completed)` | `Event::tool_completed()` |
| `ToolCallFailed` | `(Tool, Failed)` | `Event::tool_failed()` |
| `AgentToolLoopDetected` | `(System, Progress)` | `Event::system_progress()` |
| `TransformStepCompleted` | `(WorkflowStep, Completed)` | `Event::workflow_step_completed()` |
| `CustomEvent` | Any scope + type combo | `Event::new()` |

---

## Component ID Formats

Component IDs now follow enforced formats validated at event creation:

| Scope | Format | Examples |
|-------|--------|----------|
| **Workflow** | `workflow_name` | `"analysis"`, `"data_pipeline"` |
| **WorkflowStep** | `workflow:step:N` | `"analysis:step:0"`, `"pipeline:step:2"` |
| **Agent** | `agent_name` | `"researcher"`, `"summarizer"` |
| **LlmRequest** | `agent:llm:N` | `"researcher:llm:0"`, `"summarizer:llm:3"` |
| **Tool** | `tool_name` or `tool_name:N` | `"calculator"`, `"web_search:2"` |
| **System** | `system:subsystem` | `"system:tool_loop_detection"` |

**Invalid component IDs will be rejected** with a detailed error message.

---

## Step-by-Step Migration

### Step 1: Update Dependencies
```toml
[dependencies]
agent-runtime = "0.3.0"
```

### Step 2: Update Imports
```rust
// Add new types to imports
use agent_runtime::prelude::*;
use agent_runtime::{EventScope, EventType, ComponentStatus};  // NEW
```

### Step 3: Update Event Matching

**Before:**
```rust
while let Some(event) = rx.recv().await {
    match event.event_type {
        EventType::AgentLlmStreamChunk => {
            print!("{}", event.data["chunk"]);
        }
        EventType::ToolCallCompleted => {
            println!("Tool finished");
        }
        _ => {}
    }
}
```

**After:**
```rust
while let Some(event) = rx.recv().await {
    match (event.scope, event.event_type) {
        (EventScope::LlmRequest, EventType::Progress) => {
            print!("{}", event.data["chunk"]);
        }
        (EventScope::Tool, EventType::Completed) => {
            println!("Tool finished: {}", event.component_id);
        }
        _ => {}
    }
}
```

### Step 4: Update Event Creation (if applicable)

**Before:**
```rust
stream.append(
    EventType::ToolCallStarted,
    Some("my_workflow".to_string()),
    json!({"tool": "calculator"}),
).await?;
```

**After:**
```rust
// Option A: Helper method (recommended)
stream.tool_started(
    "calculator",
    Some("my_workflow".to_string()),
    None,
    json!({"args": {"a": 5}}),
).await;

// Option B: Raw append
stream.append(
    EventScope::Tool,
    EventType::Started,
    "calculator".to_string(),
    ComponentStatus::Running,
    None,
    Some("my_workflow".to_string()),
    None,
    json!({"args": {"a": 5}}),
).await;
```

### Step 5: Update Custom Event Handling

If you created custom events, map them to appropriate scopes:

**Before:**
```rust
Event {
    event_type: EventType::CustomEvent,
    data: json!({"custom_field": "value"}),
    ..
}
```

**After:**
```rust
// Choose appropriate scope based on context
Event::system_progress(
    "system:custom_subsystem",
    Some("Custom operation".to_string()),
    None,
    None,
    json!({"custom_field": "value"}),
)
```

---

## Common Migration Patterns

### Pattern 1: Workflow Event Listener

**Before:**
```rust
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        match event.event_type {
            EventType::WorkflowStarted => println!("▶ Workflow started"),
            EventType::WorkflowStepStarted => {
                println!("  Step: {}", event.data["step_name"]);
            }
            EventType::WorkflowCompleted => println!("✓ Workflow done"),
            _ => {}
        }
    }
});
```

**After:**
```rust
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        match (event.scope, event.event_type) {
            (EventScope::Workflow, EventType::Started) => {
                println!("▶ Workflow started: {}", event.component_id);
            }
            (EventScope::WorkflowStep, EventType::Started) => {
                println!("  Step: {}", event.component_id);
            }
            (EventScope::Workflow, EventType::Completed) => {
                println!("✓ Workflow done");
            }
            _ => {}
        }
    }
});
```

### Pattern 2: LLM Streaming

**Before:**
```rust
match event.event_type {
    EventType::AgentLlmStreamChunk => {
        let chunk = event.data["chunk"].as_str().unwrap();
        print!("{}", chunk);
        io::stdout().flush().unwrap();
    }
    _ => {}
}
```

**After:**
```rust
match (event.scope, event.event_type) {
    (EventScope::LlmRequest, EventType::Progress) => {
        if let Some(chunk) = event.data["chunk"].as_str() {
            print!("{}", chunk);
            io::stdout().flush().unwrap();
        }
    }
    _ => {}
}
```

### Pattern 3: Tool Execution Tracking

**Before:**
```rust
match event.event_type {
    EventType::ToolCallStarted => {
        let tool = &event.data["tool_name"];
        let args = &event.data["arguments"];
        println!("🔧 Calling {}: {:?}", tool, args);
    }
    EventType::ToolCallCompleted => {
        let result = &event.data["result"];
        println!("✓ Tool result: {}", result);
    }
    EventType::ToolCallFailed => {
        let error = &event.data["error"];
        eprintln!("✗ Tool failed: {}", error);
    }
    _ => {}
}
```

**After:**
```rust
match (event.scope, event.event_type) {
    (EventScope::Tool, EventType::Started) => {
        let args = &event.data["arguments"];
        println!("🔧 Calling {}: {:?}", event.component_id, args);
    }
    (EventScope::Tool, EventType::Completed) => {
        let result = &event.data["result"];
        println!("✓ {} result: {}", event.component_id, result);
    }
    (EventScope::Tool, EventType::Failed) => {
        let error = event.message.as_deref().unwrap_or("Unknown error");
        eprintln!("✗ {} failed: {}", event.component_id, error);
    }
    _ => {}
}
```

### Pattern 4: Component Status Tracking

**New in v0.3.0** - Track component status across events:

```rust
use std::collections::HashMap;

let mut statuses = HashMap::new();

while let Some(event) = rx.recv().await {
    // Update status tracking
    statuses.insert(event.component_id.clone(), event.status);
    
    // Handle state transitions
    match event.status {
        ComponentStatus::Running => {
            println!("▶ {} running", event.component_id);
        }
        ComponentStatus::Completed => {
            println!("✓ {} completed", event.component_id);
        }
        ComponentStatus::Failed => {
            let msg = event.message.as_deref().unwrap_or("Unknown");
            eprintln!("✗ {} failed: {}", event.component_id, msg);
        }
        _ => {}
    }
}
```

---

## Helper Method Reference

All helper methods follow a consistent signature pattern:

```rust
// Basic lifecycle events (no message)
Event::agent_started(component_id, workflow_id, parent_workflow_id, data)
Event::agent_completed(component_id, workflow_id, parent_workflow_id, data)
Event::agent_failed(component_id, error_message, workflow_id, parent_workflow_id, data)

// Progress events (with message)
Event::llm_progress(component_id, message, workflow_id, parent_workflow_id, data)
Event::tool_progress(component_id, message, workflow_id, parent_workflow_id, data)
Event::system_progress(component_id, message, workflow_id, parent_workflow_id, data)

// Complete list
Event::workflow_started()
Event::workflow_completed()
Event::workflow_failed()
Event::workflow_step_started()
Event::workflow_step_completed()
Event::workflow_step_failed()
Event::agent_started()
Event::agent_progress()
Event::agent_completed()
Event::agent_failed()
Event::llm_started()
Event::llm_progress()
Event::llm_completed()
Event::llm_failed()
Event::tool_started()
Event::tool_progress()
Event::tool_completed()
Event::tool_failed()
Event::system_progress()
```

---

## Validation and Error Handling

### Component ID Validation

v0.3.0 enforces component ID formats. Invalid IDs are rejected:

```rust
// ✓ Valid
Event::tool_started("calculator", None, None, json!({})).await;
Event::llm_started("agent:llm:0", None, None, json!({})).await;

// ✗ Invalid - will return error
Event::tool_started("", None, None, json!({})).await;
// Error: "Component ID cannot be empty"

Event::llm_started("invalid-format", None, None, json!({})).await;
// Error: "LlmRequest component_id must match 'agent_name:llm:N'"

Event::workflow_step_started("workflow:step:invalid", None, None, json!({})).await;
// Error: "WorkflowStep component_id must match 'workflow_name:step:N' where N is a number"
```

### Error Messages

The new `message` field provides human-readable context:

```rust
// Recommended: Set message for failed events
Event::agent_failed(
    "researcher",
    "API rate limit exceeded".to_string(),  // error message
    Some("workflow_1".to_string()),
    None,
    json!({"retry_after": 60}),
).await;

// Access message in handler
match (event.scope, event.event_type) {
    (_, EventType::Failed) => {
        if let Some(msg) = event.message {
            eprintln!("Error: {}", msg);
        }
    }
    _ => {}
}
```

---

## Testing Migration

Update your test assertions:

**Before:**
```rust
#[tokio::test]
async fn test_workflow_events() {
    let event = rx.recv().await.unwrap();
    assert_eq!(event.event_type, EventType::WorkflowStarted);
}
```

**After:**
```rust
#[tokio::test]
async fn test_workflow_events() {
    let event = rx.recv().await.unwrap();
    assert_eq!(event.scope, EventScope::Workflow);
    assert_eq!(event.event_type, EventType::Started);
    assert_eq!(event.status, ComponentStatus::Running);
}
```

---

## Performance Considerations

### Event Emission is Async (Non-Blocking)

v0.3.0 emits events asynchronously via `tokio::spawn()`:

```rust
// Returns immediately, event emitted in background
let handle = stream.append(...).await;

// Optional: await completion if needed
let event = handle.await.unwrap()?;
```

**Recommendation**: Ignore return value for fire-and-forget (most cases).

### Streaming Batching (Future v0.4.0)

Component ID formats enable future streaming optimizations:
- LLM chunks batched over 50ms windows
- Tool progress batched over 100ms windows
- Reduces event overhead for high-frequency updates

---

## Troubleshooting

### Issue: "Component ID cannot be empty"
**Cause**: Passing empty string to component_id  
**Fix**: Provide valid component identifier

### Issue: "component_id must match 'X:Y:N'"
**Cause**: Component ID doesn't follow required format for scope  
**Fix**: See [Component ID Formats](#component-id-formats) table

### Issue: Pattern match not exhaustive
**Cause**: Using old single-field match on `event.event_type`  
**Fix**: Switch to `(event.scope, event.event_type)` tuple matching

### Issue: Missing field errors
**Cause**: Accessing removed fields like `agent_name`  
**Fix**: Use `component_id` instead. For scope-specific info, check `event.scope`

### Issue: Helper method not found
**Cause**: Trying to use old event creation patterns  
**Fix**: Import new types: `use agent_runtime::{EventScope, EventType, ComponentStatus}`

---

## Backwards Compatibility

**None.** v0.3.0 is a breaking release. All event-related code must be updated.

To ease migration:
1. Update dependencies to `0.3.0`
2. Fix compilation errors (missing fields, changed enums)
3. Update pattern matching to `(scope, type)` tuples
4. Run tests to catch runtime issues
5. Use helper methods to simplify event creation

---

## Need Help?

- See [EVENT_STREAMING.md](EVENT_STREAMING.md) for comprehensive event system guide
- Check [CHANGELOG.md](../CHANGELOG.md) for complete list of changes
- Review updated examples in `src/bin/` directory
- File issues at project repository

---

## Summary Checklist

- [ ] Updated `agent-runtime` dependency to `0.3.0`
- [ ] Added imports for `EventScope`, `EventType`, `ComponentStatus`
- [ ] Changed event matching from `event.event_type` to `(event.scope, event.event_type)`
- [ ] Updated event field access (`agent_name` → `component_id`, added `status`, `message`)
- [ ] Replaced custom event creation with helper methods
- [ ] Updated component IDs to follow enforced formats
- [ ] Updated tests and assertions
- [ ] Verified all compilation errors resolved
- [ ] Ran test suite successfully

**Welcome to v0.3.0!** 🎉
