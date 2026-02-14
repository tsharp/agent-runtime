# Tool Loop Prevention

One common problem with LLM-based agents is **tool call loops**: the LLM calls the same tool with identical arguments repeatedly, wasting tokens and time.

## The Problem

Consider this scenario:
```
LLM: I'll search for "Rust async programming"
Tool: search("Rust async programming") → { results: [] }  # No results
LLM: Let me try searching again
Tool: search("Rust async programming") → { results: [] }
LLM: One more time...
Tool: search("Rust async programming") → { results: [] }
... (continues until max_iterations)
```

The LLM doesn't realize it's already tried this exact search and keeps retrying the same call.

## The Solution

Agent-runtime includes **automatic tool loop detection** that:
1. **Tracks tool calls** - Remembers tool name + arguments for each call
2. **Detects duplicates** - Identifies when the same tool+args are called again
3. **Injects helpful message** - Instead of executing the tool, returns a message reminding the LLM of the previous result
4. **Emits event** - Fires `AgentToolLoopDetected` for observability

## How It Works

### Detection Algorithm
1. Before executing a tool, agent checks if `(tool_name, arguments)` was already called
2. Arguments are serialized to JSON and hashed with MD5 for fast comparison
3. If duplicate detected:
   - Skip tool execution
   - Inject custom or default message as tool result
   - Emit `AgentToolLoopDetected` event
4. If not duplicate:
   - Execute tool normally
   - Record `(tool_name, arguments, result)` in tracker

### Default Behavior
Loop detection is **enabled by default** with this message:
```
I notice I'm calling {tool_name} again with the same parameters. 
The previous result was: {previous_result}
I should use this result instead of calling the tool again.
```

## Configuration

### Enable with Custom Message
```rust
use agent_runtime::{AgentConfig, ToolLoopDetectionConfig};

let agent = AgentConfig::new("assistant")
    .with_tool_loop_detection(
        ToolLoopDetectionConfig::new()
            .with_custom_message(
                "You already called {tool_name} and got: {previous_result}. Use this data."
            )
    )
    .build();
```

### Disable for Specific Agent
```rust
let agent = AgentConfig::new("explorer")
    .disable_tool_loop_detection()
    .build();
```

### YAML Configuration
```yaml
agents:
  - name: searcher
    tool_loop_detection:
      enabled: true
      custom_message: "Previous {tool_name} returned: {previous_result}"
      
  - name: unrestricted
    tool_loop_detection:
      enabled: false
```

## Message Placeholders

Custom messages support two placeholders:

### `{tool_name}`
Replaced with the name of the tool being called:
```rust
.with_custom_message("Stop calling {tool_name}!")
// → "Stop calling search_database!"
```

### `{previous_result}`
Replaced with the JSON result from the previous identical call:
```rust
.with_custom_message("Result: {previous_result}")
// → "Result: {\"results\": [], \"status\": \"success\"}"
```

## Events

When a loop is detected, an `AgentToolLoopDetected` event is emitted:

```rust
let (tx, mut rx) = mpsc::channel(100);

tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        if event.event_type == EventType::AgentToolLoopDetected {
            println!("🔁 Loop detected!");
            println!("  Tool: {}", event.data["tool_name"]);
            println!("  Args: {}", event.data["arguments"]);
            println!("  Previous result: {}", event.data["previous_result"]);
        }
    }
});

agent.execute_with_events(&input, &tx).await?;
```

## Enhanced ToolResult

Tools can help prevent loops by signaling when they have no data:

```rust
use agent_runtime::{Tool, ToolResult, ToolStatus};

async fn search_tool(args: Value) -> ToolResult {
    let results = search_database(&args["query"]).await;
    
    if results.is_empty() {
        // Signal "no data" to prevent LLM from retrying
        ToolResult::success_no_data()
            .with_message("No results found for this query.")
    } else {
        ToolResult::success(serde_json::to_value(&results).unwrap())
    }
}
```

### ToolStatus Enum
- **`Success`** - Tool executed successfully with data
- **`SuccessNoData`** - Tool executed successfully but returned no data (hints to LLM to try different approach)
- **`Error`** - Tool execution failed

### Helper Methods
```rust
// Success with data
ToolResult::success(json!({"result": 42}))

// Success but empty/null result
ToolResult::success_no_data()
    .with_message("No data available")

// Error
ToolResult::error("Database connection failed")
```

## Example: Preventing Search Loops

**Without loop prevention:**
```
User: Find information about "quantum computing"
LLM → search("quantum computing") → []
LLM → search("quantum computing") → []
LLM → search("quantum computing") → []
... 7 more times ...
LLM: I couldn't find any information
```

**With loop prevention:**
```
User: Find information about "quantum computing"
LLM → search("quantum computing") → []
LLM → search("quantum computing") → LOOP DETECTED
Agent: "I already called search and got: []. I should try a different approach."
LLM: Let me try a different search term
LLM → search("quantum mechanics basics") → [results...]
```

## When to Disable

Loop detection should be disabled when:
1. **Intentional retries** - Tool is expected to be called multiple times with same args (e.g., polling)
2. **State-changing tools** - Tool modifies state, so repeated calls are valid (e.g., increment counter)
3. **Time-sensitive tools** - Results change over time (e.g., get current time)

Example:
```rust
// Polling tool - disable loop detection
let poller = AgentConfig::new("status_checker")
    .disable_tool_loop_detection()
    .with_tool(check_status_tool())
    .build();
```

## Testing

Loop detection includes comprehensive tests:

```bash
cargo test tool_loop_detection
```

Test coverage:
- ✅ Detects duplicate tool calls with same arguments
- ✅ Allows different arguments to same tool
- ✅ Allows same arguments to different tools
- ✅ Custom messages with placeholder replacement
- ✅ Event emission on loop detection

## Best Practices

1. **Keep enabled by default** - Prevents most accidental loops
2. **Customize messages for domain** - Help LLM understand context
3. **Use `success_no_data()` in tools** - Signal when search/query finds nothing
4. **Monitor events** - Track loop detection to identify problematic tool patterns
5. **Disable selectively** - Only disable for specific agents that need retries

## Performance Impact

Loop detection has minimal overhead:
- **MD5 hashing** - Fast argument comparison (~microseconds)
- **Memory** - Stores call history per agent execution (cleared after completion)
- **No network** - All detection is local, no external calls

The savings from **prevented duplicate calls** far outweigh the detection cost.
