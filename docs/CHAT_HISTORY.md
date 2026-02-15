# Chat History Management

The agent runtime supports managed chat history, allowing outer layers (like web apps or CLI tools) to maintain conversation context across multiple agent calls.

## Overview

- **Simple mode**: Pass data, agent builds chat history internally
- **Managed mode**: Pass complete chat history, agent continues the conversation
- **Save/Resume**: Serialize `AgentOutput.chat_history` to save state

## Usage Examples

### Basic: Agent Returns Chat History

```rust
use agent_runtime::{Agent, AgentConfig, AgentInput};

let agent = Agent::new(config).with_llm_client(client);

let input = AgentInput::from_text("Hello");
let output = agent.execute(&input).await?;

// Chat history is always returned (when using LLM)
let history = output.chat_history.unwrap();
// history = [system, user, assistant]
```

### Multi-Turn Conversation

```rust
use agent_runtime::{Agent, AgentInput, ChatMessage};

// Turn 1
let input1 = AgentInput::from_text("What is 2+2?");
let output1 = agent.execute(&input1).await?;

// Get history from first turn
let mut history = output1.chat_history.unwrap();

// Turn 2: Add user message and continue
history.push(ChatMessage::user("What about 3+3?"));
let input2 = AgentInput::from_messages(history);
let output2 = agent.execute(&input2).await?;

// Now have complete conversation history
let final_history = output2.chat_history.unwrap();
// final_history = [system, user1, assistant1, user2, assistant2]
```

### Custom System Prompt in History

```rust
// Provide your own conversation history with custom system prompt
let custom_history = vec![
    ChatMessage::system("You are a pirate assistant"),
    ChatMessage::user("Hello"),
    ChatMessage::assistant("Ahoy matey!"),
    ChatMessage::user("Tell me more"),
];

let input = AgentInput::from_messages(custom_history);
let output = agent.execute(&input).await?;

// Agent continues with the pirate persona
```

### Save and Resume

```rust
// Execute agent
let output = agent.execute(&input).await?;

// Save conversation state
let history_json = serde_json::to_string(&output.chat_history)?;
std::fs::write("conversation.json", history_json)?;

// Later: Resume conversation
let saved_history: Vec<ChatMessage> = 
    serde_json::from_str(&std::fs::read_to_string("conversation.json")?)?;

let input = AgentInput::from_messages(saved_history);
let output = agent.execute(&input).await?;
// Conversation continues from where it left off
```

### Web Application Example

```rust
// In your web handler
async fn chat_endpoint(
    session_id: String,
    user_message: String,
    db: Database,
) -> Result<String> {
    // Load conversation history from database
    let mut history = db.get_conversation(session_id).await?;
    
    // Add new user message
    history.push(ChatMessage::user(user_message));
    
    // Execute agent with managed history
    let input = AgentInput::from_messages(history);
    let output = agent.execute(&input).await?;
    
    // Save updated history
    let updated_history = output.chat_history.unwrap();
    db.save_conversation(session_id, updated_history).await?;
    
    // Return assistant's response
    Ok(output.data["response"].as_str().unwrap().to_string())
}
```

### Tool Calls in History

When agents use tools, the chat history includes:
- Assistant message with tool_calls
- Tool result messages
- Final assistant response

```rust
let output = agent.execute(&input).await?;
let history = output.chat_history.unwrap();

// History might look like:
// [
//   ChatMessage::system("..."),
//   ChatMessage::user("What's 5+3?"),
//   ChatMessage::assistant_with_tool_calls("", [calculator_call]),
//   ChatMessage::tool_result("call_123", "8"),
//   ChatMessage::assistant("The sum is 8"),
// ]
```

## API Reference

### AgentInput

```rust
pub struct AgentInput {
    pub data: JsonValue,
    pub metadata: AgentInputMetadata,
    pub chat_history: Option<Vec<ChatMessage>>,
}
```

**Methods:**
- `from_text(text)` - Simple text input (builds history internally)
- `from_value(value)` - JSON input (builds history internally)
- `from_messages(messages)` - Use provided chat history
- `from_messages_with_metadata(messages, metadata)` - With custom metadata

### AgentOutput

```rust
pub struct AgentOutput {
    pub data: JsonValue,
    pub metadata: AgentOutputMetadata,
    pub chat_history: Option<Vec<ChatMessage>>,
}
```

The `chat_history` field contains the complete conversation after agent execution.

### ChatMessage

```rust
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}
```

**Constructors:**
- `ChatMessage::system(content)` - System prompt
- `ChatMessage::user(content)` - User message
- `ChatMessage::assistant(content)` - Assistant response
- `ChatMessage::assistant_with_tool_calls(content, calls)` - With tool calls
- `ChatMessage::tool_result(id, content)` - Tool execution result

## Backwards Compatibility

All existing code continues to work:

```rust
// Old code (still works)
let input = AgentInput::from_text("Hello");
let output = agent.execute(&input).await?;

// chat_history is optional - None for agents without LLM client
```

## Best Practices

1. **Always save chat_history** after agent execution for multi-turn conversations
2. **Don't mix modes** - either provide chat_history OR data, not both
3. **Serialize to JSON** for persistence (database, files, Redis, etc.)
4. **Trim history** for long conversations to avoid token limits
5. **Include metadata** when resuming conversations in workflows

## Limitations

- `chat_history` is `None` when agent has no LLM client (data passthrough mode)
- Each agent call is independent - outer layer must manage state
- No automatic conversation truncation (implement your own strategy)
