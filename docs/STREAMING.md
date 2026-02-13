# LLM Streaming Implementation

## What Changed

### Added Streaming Support to ChatClient Trait
```rust
pub type TextStream = Pin<Box<dyn Stream<Item = LlmResult<String>> + Send>>;

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> LlmResult<ChatResponse>;
    async fn chat_stream(&self, request: ChatRequest) -> LlmResult<TextStream>;
    fn model(&self) -> &str;
    fn provider(&self) -> &str;
}
```

### Implemented Streaming for LlamaClient
- Parses SSE (Server-Sent Events) format from llama.cpp
- Extracts text chunks from delta responses
- Returns async stream of text chunks

### New Event Type
- `AgentLlmStreamChunk` - Emitted for each text chunk received

### Agent Streaming Execution
- `Agent::execute_with_events()` now uses `chat_stream()` instead of `chat()`
- Collects chunks into full response
- Emits `AgentLlmStreamChunk` event for each chunk
- Final response same as before

### Simplified Workflow Demo Output
**Before:**
```
💬 LLM Request Started (greeter)
   [system]: You are a friendly greeter...
   [user]: Hello! I'm interested in learning about AI agents.
✅ LLM Response (greeter)
   Hello! Welcome! I'm delighted to meet you...
   Tokens: 245
```

**After:**
```
🤖 greeter >
   Hello! Welcome! I'm delighted to meet you...
   (text streams character by character as it arrives)
```

Much cleaner and shows the streaming in action!

## How It Works

### 1. LLM Server Side
llama.cpp sends SSE format:
```
data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" there"}}]}

data: [DONE]
```

### 2. Client Parses Stream
```rust
let stream = response.bytes_stream();
let text_stream = stream.map(|chunk_result| {
    // Parse "data: {...}" format
    // Extract delta.content
    // Return text chunk
});
```

### 3. Agent Consumes Stream
```rust
let mut text_stream = client.chat_stream(request).await?;
while let Some(chunk_result) = text_stream.next().await {
    let chunk = chunk_result?;
    full_response.push_str(&chunk);
    
    // Emit event
    event_stream.append(EventType::AgentLlmStreamChunk, ...);
}
```

### 4. Demo Displays Stream
```rust
EventType::AgentLlmStreamChunk => {
    if let Some(chunk) = event.data.get("chunk") {
        print!("{}", chunk);  // Print immediately
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
}
```

## Benefits

### Real-Time Feedback
- User sees responses as they're generated
- No waiting for complete response
- Better UX for long responses

### Observability
- Can monitor tokens as they stream
- Can stop generation early if needed
- Can track generation speed

### Error Handling
- Detect errors mid-stream
- Fail fast instead of waiting
- Better error messages

## Example Output

```
=== Workflow Demo ===

✓ LLM client configured (https://192.168.91.57 - insecure)
✓ Created 3 agents: greeter → analyzer → summarizer
✓ Workflow built with 3 sequential steps

📡 Streaming Agent Responses
============================================================

🤖 greeter >
   Well hello there! It's lovely to meet you! 😊
   
   My name is Kai, and I'm delighted to help you dive into
   the fascinating world of AI Agents!

🤖 analyzer >
   ## Analysis of the AI Agent Introduction
   
   This response is a strong, welcoming introduction from
   an AI Agent named Kai. Here's a breakdown...

🤖 summarizer >
   This analysis highlights Kai, an AI Agent, as having a
   remarkably friendly and approachable introduction...

============================================================
✅ Workflow Completed

📊 Final Results

Steps executed: 3
  1. greeter (Agent) - 5559ms
  2. analyzer (Agent) - 17383ms
  3. summarizer (Agent) - 3379ms
```

## Technical Details

### SSE Format
Server-Sent Events format from llama.cpp/OpenAI:
```
data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

### Stream Processing
```rust
for line in text.lines() {
    if let Some(json_str) = line.strip_prefix("data: ") {
        if json_str.trim() == "[DONE]" {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
            if let Some(delta) = parsed.get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
            {
                return Ok(delta.to_string());
            }
        }
    }
}
```

### Error Handling
- Network errors during streaming
- Parse errors for malformed SSE
- Early termination handling
- Timeout handling

## Current Limitations

### OpenAI Provider
- Streaming not yet implemented (returns error)
- Would need similar SSE parsing
- Authentication headers required

### Token Usage
- No usage stats in streaming mode
- Would need to count tokens client-side
- Or wait for final message with usage

### Function Calling
- Not yet implemented
- Would need to detect tool calls in stream
- Pause streaming, execute tool, resume

## Next Steps

### 1. OpenAI Streaming
Implement for OpenAI provider using same SSE parsing.

### 2. Token Counting
Add client-side token counting during streaming.

### 3. Stop Generation
Add ability to cancel/stop mid-stream:
```rust
runtime.stop_generation(workflow_id).await;
```

### 4. Function Calling
Detect and handle tool calls in streamed responses.

### 5. HTTP Streaming Endpoint
Expose event stream via HTTP:
```rust
#[get("/workflows/{id}/stream")]
async fn stream(runtime: Data<Runtime>) -> impl Responder {
    let events = runtime.event_stream().subscribe();
    // Convert to SSE response
}
```

## Usage

### Basic Streaming
```rust
let mut stream = client.chat_stream(request).await?;
while let Some(chunk) = stream.next().await {
    print!("{}", chunk?);
}
```

### With Events
```rust
let mut receiver = runtime.event_stream().subscribe();
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        if let EventType::AgentLlmStreamChunk = event.event_type {
            if let Some(chunk) = event.data.get("chunk") {
                print!("{}", chunk);
            }
        }
    }
});
```

### Run the Demo
```bash
cargo run --bin workflow_demo
```

Watch the agents stream their responses in real-time! 🚀
