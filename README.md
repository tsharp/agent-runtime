# agent-runtime

A production-ready Rust framework for building AI agent workflows with native and external tool support, streaming LLM interactions, comprehensive event tracking, and intelligent loop prevention.

## Features

### 🤖 Agent System
- **LLM-backed agents** with configurable system prompts and context
- **Multi-provider LLM support** - OpenAI and llama.cpp (LM Studio) included
- **Streaming responses** - Real-time token-by-token LLM output
- **Tool loop prevention** - Automatic detection and prevention of redundant tool calls
- **Execution history** - Complete conversation and tool call tracking per agent

### 🔧 Tool System
- **Native tools** - In-memory async functions with zero overhead
- **MCP tool integration** - Connect to external MCP servers (filesystem, databases, web, etc.)
- **Tool registry** - Organize and manage tools per agent
- **Automatic discovery** - MCP tools auto-discovered from servers
- **Rich metadata** - Full argument schemas and descriptions

### 🔄 Workflow Engine
- **Sequential workflows** - Chain multiple agents with state passing
- **Transform steps** - Data manipulation between agents
- **Conditional branching** - Dynamic workflow paths
- **Nested workflows** - SubWorkflows for complex orchestration
- **Mermaid export** - Visualize workflows as diagrams

### 📡 Event System
- **Real-time events** - Complete visibility into execution
- **Fine-grained tracking** - Workflow, agent, LLM, and tool events
- **Streaming chunks** - Live LLM token streaming via events
- **Multi-subscriber** - Multiple event listeners per workflow
- **Event bubbling** - Events propagate from tools → agents → workflows

### ⚙️ Configuration
- **YAML and TOML support** - Human-readable config files
- **Builder pattern** - Type-safe programmatic configuration
- **Environment variables** - Runtime configuration override
- **Per-agent settings** - System prompts, tools, LLM clients, loop prevention

### 🔒 Production Ready
- **61 comprehensive tests** - All core functionality tested
- **Tool loop prevention** - Prevents LLM from calling same tool repeatedly
- **Microsecond timing** - Precise performance metrics
- **Structured logging** - FileLogger with timestamped output
- **Error handling** - Detailed error types with context

## Quick Start

### Installation
```toml
[dependencies]
agent-runtime = { path = "." }
tokio = { version = "1", features = ["full"] }
```

### Basic Agent
```rust
use agent_runtime::prelude::*;

#[tokio::main]
async fn main() {
    // Create LLM client
    let llm = OpenAiClient::new("https://api.openai.com/v1", "your-api-key");
    
    // Build agent with tools
    let agent = AgentConfig::new("assistant")
        .with_system_prompt("You are a helpful assistant.")
        .with_llm_client(Arc::new(llm))
        .with_tool(calculator_tool())
        .build();
    
    // Execute
    let input = AgentInput::from_text("What is 42 * 137?");
    let output = agent.execute(&input).await?;
    println!("Result: {}", output.data);
}
```

### MCP External Tools
```rust
use agent_runtime::tools::{McpClient, McpTool};

// Connect to MCP server
let mcp = McpClient::new_stdio(
    "npx",
    vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
).await?;

// Discover tools
let tools = mcp.list_tools().await?;
println!("Available: {:?}", tools.iter().map(|t| &t.name).collect::<Vec<_>>());

// Use in agent
let agent = AgentConfig::new("file-agent")
    .with_mcp_tools(Arc::new(mcp))
    .build();
```

### Workflow
```rust
let workflow = Workflow::new("analysis")
    .add_step(AgentStep::new(researcher_agent))
    .add_step(TransformStep::new(|output| {
        // Transform data between agents
        AgentInput::from_text(format!("Summarize: {}", output.data))
    }))
    .add_step(AgentStep::new(summarizer_agent))
    .build();

let result = workflow.execute(initial_input, &mut event_rx).await?;
```

### Event Streaming
```rust
let (tx, mut rx) = mpsc::channel(100);

// Subscribe to events
tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        match event.event_type {
            EventType::AgentLlmStreamChunk => {
                print!("{}", event.data.get("chunk").unwrap());
            }
            EventType::ToolCallCompleted => {
                println!("Tool {} returned: {}", 
                    event.data["tool_name"],
                    event.data["result"]
                );
            }
            _ => {}
        }
    }
});

agent.execute_with_events(&input, &tx).await?;
```

### Configuration Files
```yaml
# agent-runtime.yaml
agents:
  - name: researcher
    system_prompt: "You are a research assistant."
    max_iterations: 10
    tool_loop_detection:
      enabled: true
      custom_message: "Previous {tool_name} call returned: {previous_result}"
    
  - name: analyzer
    system_prompt: "You analyze data."
    tool_loop_detection:
      enabled: false  # Disable if needed
```

```rust
let config = RuntimeConfig::from_file("agent-runtime.yaml")?;
```

## Architecture

### Core Modules
- **`runtime`** - Workflow execution engine with event emission
- **`workflow`** - Builder pattern for composing steps
- **`agent`** - LLM-backed agents with tool execution loop
- **`step`** - Trait for workflow steps (Agent, Transform, Conditional, SubWorkflow)
- **`llm`** - Provider-agnostic chat client (OpenAI, llama.cpp)
- **`tool`** - Native tool trait and registry
- **`tools/mcp_client`** - MCP protocol client for external tools
- **`event`** - Event types and streaming system
- **`config`** - YAML/TOML configuration loading
- **`tool_loop_detection`** - Intelligent duplicate tool call prevention

### Event Types
- **Workflow**: Started, StepStarted, StepCompleted, StepFailed, Completed, Failed
- **Agent**: Started, Completed, Failed, LlmStreamChunk
- **LLM**: RequestSent, ResponseReceived, StreamChunkReceived
- **Tool**: ToolCallStarted, ToolCallCompleted, ToolCallFailed, AgentToolLoopDetected

### Tool Loop Prevention
Prevents LLMs from calling the same tool with identical arguments repeatedly:
- **Automatic detection** - Tracks tool calls and arguments using MD5 hashing
- **Configurable messages** - Custom messages with `{tool_name}` and `{previous_result}` placeholders
- **Event emission** - `AgentToolLoopDetected` event for observability
- **Enabled by default** - Can be disabled per-agent if needed

## Examples

Run any demo:
```bash
# Workflows
cargo run --bin workflow_demo          # 3-agent workflow with LLM
cargo run --bin hello_workflow         # Simple sequential workflow
cargo run --bin nested_workflow        # SubWorkflow example

# Agents & Tools
cargo run --bin agent_with_tools_demo  # Agent with calculator & weather
cargo run --bin native_tools_demo      # Standalone native tools
cargo run --bin mcp_tools_demo         # MCP external tools

# LLM Clients
cargo run --bin llm_demo               # OpenAI client
cargo run --bin llama_demo             # llama.cpp/LM Studio

# Configuration
cargo run --bin config_demo            # YAML/TOML loading

# Visualization
cargo run --bin mermaid_viz            # Generate workflow diagrams
cargo run --bin complex_viz            # Complex workflow diagram
```

## Documentation

- **[Specification](docs/spec.md)** - Complete system design
- **[Tool Calling](docs/TOOL_CALLING.md)** - Native tool usage
- **[MCP Integration](docs/MCP_INTEGRATION.md)** - External MCP tools
- **[Event Streaming](docs/EVENT_STREAMING.md)** - Event system guide
- **[LLM Module](docs/LLM_MODULE.md)** - LLM provider integration
- **[Workflow Composition](docs/WORKFLOW_COMPOSITION.md)** - Building workflows
- **[Testing](docs/TESTING.md)** - Test suite documentation

## Testing

```bash
cargo test              # All 61 tests
cargo test --lib        # Library tests only
cargo test agent        # Agent tests
cargo test tool         # Tool tests
cargo clippy            # Linting
```

## License
Dual-licensed under MIT or Apache-2.0 at your option.
