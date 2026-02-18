# Advanced Context Management Strategies

This document describes the advanced context management strategies available in Phase 6 of the workflow chat history implementation.

## Overview

In addition to the basic strategies (TokenBudgetManager and SlidingWindowManager), Phase 6 adds two advanced strategies for sophisticated context management:

1. **MessageTypeManager** - Priority-based pruning by message type
2. **SummarizationManager** - LLM-based compression of old messages

## MessageTypeManager

### Purpose
Prioritizes messages by type and importance, keeping system prompts and recent conversation pairs while pruning less critical messages like old tool calls.

### Use Cases
- **Multi-agent workflows** where recent dialogue is critical
- **Tool-heavy conversations** with many tool calls that become less relevant over time
- **Conversational agents** that need to maintain recent context

### Configuration

```rust
use agent_runtime::MessageTypeManager;

// Keep max 20 messages, preserve last 5 user/assistant pairs
let manager = MessageTypeManager::new(20, 5);
```

**Parameters:**
- `max_messages`: Maximum total messages to keep in history
- `keep_recent_pairs`: Number of recent user/assistant conversation pairs to always preserve

### Behavior

**Priority Levels:**
1. **Critical** - System messages (always kept)
2. **High** - User and Assistant messages (preserved by recency)
3. **Low** - Tool messages (pruned first)

**Algorithm:**
1. Always preserve all system messages
2. Identify and protect the last N user/assistant pairs
3. Remove low-priority messages (tool calls) first
4. If still over limit, sort by priority and truncate

### Example

```rust
let workflow = Workflow::builder()
    .with_chat_history(Arc::new(MessageTypeManager::new(15, 3)))
    .add_step(agent1)  // Researcher
    .add_step(agent2)  // Analyst (uses tools)
    .add_step(agent3)  // Reporter
    .build();

// After execution:
// - System prompts: Preserved
// - Last 3 user/assistant pairs: Preserved
// - Old tool calls: Pruned
// - Total messages: ≤ 15
```

### Advantages
- ✅ Maintains conversation coherence
- ✅ Preserves critical system instructions
- ✅ Removes verbose tool outputs automatically
- ✅ Simple, predictable behavior

### Limitations
- Token count not considered (only message count)
- May not work well for extremely long individual messages
- Fixed priority scheme (not customizable)

## SummarizationManager

### Purpose
Compresses old conversation history into summary messages when token limits are approached, preserving recent messages intact.

### Use Cases
- **Long-running workflows** with extensive history
- **Research pipelines** where old findings should be summarized
- **Multi-stage analysis** where early stages can be compressed

### Configuration

```rust
use agent_runtime::SummarizationManager;

// Max 18k input tokens
// Trigger summarization at 15k tokens
// Target ~500 tokens for summaries
// Keep last 10 messages untouched
let manager = SummarizationManager::new(18_000, 15_000, 500, 10);
```

**Parameters:**
- `max_input_tokens`: Maximum tokens allowed for input
- `summarization_threshold`: Token count that triggers summarization
- `summary_token_target`: Target size for compressed summaries (reserved for future use)
- `keep_recent_count`: Number of recent messages to preserve unsummarized

### Behavior

**Algorithm:**
1. Monitor total token count
2. When exceeds threshold:
   - Split history into "old" (to summarize) and "recent" (keep as-is)
   - Preserve system messages from old section
   - Create summary of non-system old messages
   - Combine: system messages + summary + recent messages
3. If still over limit, apply emergency truncation

**Summary Format:**
```text
Summary of previous conversation:

- 5 user inputs and 5 assistant responses
- Initial topic: Analyze Q4 sales data and identify trends...
- Latest response: Based on the analysis, I recommend increasing...

[This is a compressed summary. Original messages were removed to save context space.]
```

### Example

```rust
let workflow = Workflow::builder()
    .with_chat_history(Arc::new(SummarizationManager::new(
        18_000,  // Max input tokens
        15_000,  // Trigger at 15k
        500,     // Summary target
        10       // Keep last 10 messages
    )))
    .add_step(researcher)      // Stage 1
    .add_step(analyzer)        // Stage 2
    .add_step(deep_analyzer)   // Stage 3
    .add_step(reporter)        // Stage 4
    .build();

// After execution with 30+ messages:
// - System prompts: Preserved
// - Messages 1-20: Summarized into compact summary
// - Messages 21-30: Kept verbatim
// - Final message count: ~12 messages (system + summary + last 10)
```

### Advantages
- ✅ Preserves information from old messages
- ✅ Keeps recent context intact
- ✅ Token-aware (not just message count)
- ✅ Handles very long workflows

### Limitations
- Current implementation uses template-based summaries (not LLM-generated)
- Summary quality depends on implementation
- Adds computational overhead (when enhanced with LLM calls)
- May lose nuance from original messages

### Future Enhancements

The `summary_token_target` parameter is reserved for future LLM-based summarization:

```rust
// Future enhancement: Call LLM to create intelligent summaries
async fn create_llm_summary(
    messages: &[ChatMessage],
    target_tokens: usize,
    llm_client: &dyn ChatClient
) -> ChatMessage {
    let prompt = format!(
        "Summarize the following conversation in approximately {} tokens:\n\n{}",
        target_tokens,
        format_messages(messages)
    );
    
    let summary = llm_client.complete(prompt).await?;
    ChatMessage::system(summary)
}
```

## Strategy Comparison

| Feature | TokenBudget | SlidingWindow | MessageType | Summarization |
|---------|-------------|---------------|-------------|---------------|
| **Metric** | Tokens | Message count | Message count + type | Tokens |
| **Pruning** | Oldest first | FIFO | Priority-based | Compression |
| **Preserves** | System + recent | Recent only | System + pairs | System + recent |
| **Best For** | General use | Simple cases | Multi-agent | Long workflows |
| **Overhead** | Low | Very low | Low | Medium |
| **Information Loss** | High | High | Medium | Low |

## Choosing a Strategy

### Use **TokenBudgetManager** when:
- You need flexible token management (any context size/ratio)
- Simple pruning is sufficient
- General-purpose workflows

### Use **SlidingWindowManager** when:
- You want predictable, simple behavior
- Message count matters more than tokens
- Stateless or short workflows

### Use **MessageTypeManager** when:
- You have multi-agent conversations
- Tool calls create noise in history
- Recent dialogue is most important
- You want to maintain conversation coherence

### Use **SummarizationManager** when:
- Workflows can become very long
- Old context should be compressed, not discarded
- You need to preserve information over time
- Token limits are strict

## Combining Strategies

While workflows use one strategy at a time, you can chain strategies externally:

```rust
// Example: Apply MessageType first, then Summarization
let checkpoint1 = workflow1.execute_with(MessageTypeManager::new(30, 5)).await;
let restored = deserialize(checkpoint1);

let workflow2 = Workflow::builder()
    .with_restored_context(restored)
    .with_chat_history(SummarizationManager::new(18_000, 15_000, 500, 10))
    .add_step(next_agent)
    .build();
```

## Performance Considerations

### MessageTypeManager
- **Time Complexity**: O(n log n) for sorting protected messages
- **Space Complexity**: O(n) for tracking indices
- **Best Case**: Few messages, no pruning needed
- **Worst Case**: Many messages, frequent pruning

### SummarizationManager
- **Time Complexity**: O(n) for splitting and filtering
- **Space Complexity**: O(n) for creating new history
- **Best Case**: Below threshold, no summarization
- **Worst Case**: Frequent summarization with LLM calls (future)

## Testing

Both strategies include comprehensive test coverage:

### MessageTypeManager Tests
- Creation and configuration
- Should-prune logic
- Priority-based pruning
- System message preservation
- Recent pair extraction

### SummarizationManager Tests
- Creation and configuration
- Threshold-based pruning
- Summary generation
- Recent message preservation
- System message handling
- Emergency truncation

## Demonstration

Run the comprehensive demo:

```bash
cargo run --bin advanced_strategies_demo
```

This demonstrates:
1. MessageTypeManager with multi-agent workflow
2. SummarizationManager with multi-stage pipeline
3. Side-by-side strategy comparison

## API Reference

### MessageTypeManager

```rust
impl MessageTypeManager {
    pub fn new(max_messages: usize, keep_recent_pairs: usize) -> Self;
}

#[async_trait]
impl ContextManager for MessageTypeManager {
    async fn should_prune(&self, history: &[ChatMessage], _: usize) -> bool;
    async fn prune(&self, history: Vec<ChatMessage>) 
        -> Result<(Vec<ChatMessage>, usize), ContextError>;
    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize;
    fn name(&self) -> &str;
}
```

### SummarizationManager

```rust
impl SummarizationManager {
    pub fn new(
        max_input_tokens: usize,
        summarization_threshold: usize,
        summary_token_target: usize,
        keep_recent_count: usize
    ) -> Self;
}

#[async_trait]
impl ContextManager for SummarizationManager {
    async fn should_prune(&self, _: &[ChatMessage], current_tokens: usize) -> bool;
    async fn prune(&self, history: Vec<ChatMessage>) 
        -> Result<(Vec<ChatMessage>, usize), ContextError>;
    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize;
    fn name(&self) -> &str;
}
```

## Summary

Phase 6 adds sophisticated context management for advanced use cases:

- **MessageTypeManager**: Intelligent priority-based pruning
- **SummarizationManager**: Compression instead of deletion
- **Comprehensive tests**: 15 tests covering all scenarios
- **Demo application**: Real-world examples

These strategies complement the basic strategies to provide a complete toolkit for workflow context management.
