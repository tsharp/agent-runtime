# Event Streaming & Observability

## What Was Added

### LLM-Specific Events
Added three new event types to track LLM interactions:
- `AgentLlmRequestStarted` - Emitted when agent sends request to LLM
- `AgentLlmRequestCompleted` - Emitted when LLM responds successfully
- `AgentLlmRequestFailed` - Emitted when LLM call fails

### Execution Context
Created `ExecutionContext` to pass runtime context (like event streams) to steps:
```rust
pub struct ExecutionContext<'a> {
    pub event_stream: Option<&'a EventStream>,
}
```

### Agent Event Emission
Updated `Agent::execute_with_events()` to emit detailed events:
- Agent processing started
- LLM request with full message history
- LLM response with content and usage
- Agent completion or failure

### Step Trait Enhancement
Added `execute_with_context()` method to Step trait:
```rust
#[async_trait]
pub trait Step {
    async fn execute_with_context(&self, input: StepInput, ctx: ExecutionContext<'_>) -> StepResult;
    async fn execute(&self, input: StepInput) -> StepResult {
        self.execute_with_context(input, ExecutionContext::new()).await
    }
}
```

### Runtime Event Access
Exposed `event_stream()` method on Runtime for subscribing to events:
```rust
let mut receiver = runtime.event_stream().subscribe();
while let Ok(event) = receiver.recv().await {
    // Handle event
}
```

## Enhanced Workflow Demo

The workflow demo now includes real-time event monitoring:

### Features
- **Live Event Stream**: Separate task subscribes to runtime events
- **Detailed LLM Visibility**: See every message sent to and from LLM
- **Token Usage**: Track token consumption per request
- **Execution Timing**: Monitor step and agent execution times
- **Error Handling**: See detailed error messages when things fail

### Output Example
```
=== Workflow Demo ===

✓ LLM client configured (https://192.168.91.57 - insecure)
✓ Created 3 agents: greeter → analyzer → summarizer
✓ Workflow built with 3 sequential steps

📡 Event Stream Monitor Started
============================================================

🚀 Workflow Started
------------------------------------------------------------

▶  Step Started: greeter
   🤖 Agent 'greeter' processing...
   💬 LLM Request Started (greeter)
      [system]: You are a friendly greeter. Say hello and introduce yourself warmly.
      [user]: Hello! I'm interested in learning about AI agents.
   ✅ LLM Response (greeter)
      Hello! Welcome! I'm delighted to meet you...
      Tokens: 245
   ⏱  Step Completed: greeter (1234ms)

▶  Step Started: analyzer
   🤖 Agent 'analyzer' processing...
   💬 LLM Request Started (analyzer)
      [system]: You are a thoughtful analyzer...
      [user]: {"response": "Hello! Welcome..."}
   ✅ LLM Response (analyzer)
      This is a warm greeting that demonstrates...
      Tokens: 312
   ⏱  Step Completed: analyzer (1567ms)

▶  Step Started: summarizer
   🤖 Agent 'summarizer' processing...
   💬 LLM Request Started (summarizer)
      [system]: You are a concise summarizer...
      [user]: {"response": "This is a warm greeting..."}
   ✅ LLM Response (summarizer)
      The conversation consisted of...
      Tokens: 89
   ⏱  Step Completed: summarizer (876ms)

============================================================
✅ Workflow Completed

============================================================

📊 Final Results

Output:
{
  "response": "The conversation consisted of..."
}

Steps executed: 3
  1. greeter (agent) - 1234ms
  2. analyzer (agent) - 1567ms
  3. summarizer (agent) - 876ms
```

## Benefits

### 1. **Complete Observability**
- See exactly what's happening inside agents
- Monitor LLM calls in real-time
- Track token usage per request

### 2. **Better Debugging**
- Detailed error messages with context
- Full conversation history visible
- Execution timing for performance tuning

### 3. **Production Ready**
- Events can be streamed to monitoring systems
- Event history preserved for replay
- Offset-based replay for reconnection

### 4. **Streaming Potential**
- Foundation for HTTP/WebSocket streaming
- Can add event filtering by type
- Can add event persistence/storage

## Event Types Reference

### Workflow Events
- `WorkflowStarted` - Workflow execution begins
- `WorkflowStepStarted` - Step starts executing
- `WorkflowStepCompleted` - Step finishes successfully
- `WorkflowCompleted` - Workflow finishes successfully
- `WorkflowFailed` - Workflow encounters error

### Agent Events
- `AgentProcessing` - Agent begins processing
- `AgentCompleted` - Agent finishes successfully
- `AgentFailed` - Agent encounters error

### LLM Events (New!)
- `AgentLlmRequestStarted` - LLM request sent (includes messages)
- `AgentLlmRequestCompleted` - LLM responds (includes response & usage)
- `AgentLlmRequestFailed` - LLM call fails (includes error)

### Tool Events
- `ToolCallStarted` - Tool execution starts
- `ToolCallCompleted` - Tool execution completes
- `ToolCallFailed` - Tool execution fails

## Usage

### Subscribe to Events
```rust
let runtime = Runtime::new();
let mut receiver = runtime.event_stream().subscribe();

// In a separate task
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        println!("{:?}", event);
    }
});
```

### Filter Events by Type
```rust
while let Ok(event) = receiver.recv().await {
    match event.event_type {
        EventType::AgentLlmRequestCompleted => {
            // Handle LLM responses
        }
        EventType::WorkflowFailed => {
            // Handle errors
        }
        _ => {}
    }
}
```

### Access Event Data
```rust
if let Some(response) = event.data.get("response").and_then(|v| v.as_str()) {
    println!("LLM said: {}", response);
}
```

## Next Steps

### HTTP Event Streaming Endpoint
Add actix-web route to stream events:
```rust
#[get("/workflows/{id}/stream")]
async fn stream_events(runtime: Data<Runtime>) -> HttpResponse {
    let stream = runtime.event_stream().subscribe();
    // Convert to SSE or chunked response
}
```

### Event Persistence
Store events in database for:
- Historical analysis
- Debugging past runs
- Audit trail
- Cost tracking

### Event Filtering
Add subscription filters:
```rust
runtime.event_stream()
    .subscribe()
    .filter(|e| matches!(e.event_type, EventType::AgentLlm*))
```

### Metrics & Analytics
Aggregate events for:
- Token usage per workflow/agent
- Average execution times
- Error rates
- Cost calculations
