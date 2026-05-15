# agent-runtime

[![Crates.io](https://img.shields.io/crates/v/agent-runtime.svg)](https://crates.io/crates/agent-runtime)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A Rust framework for building AI agent workflows with tools, streaming LLM responses,
event tracking, and intelligent tool-loop prevention.

## Features

- **Agents** backed by pluggable LLM providers (OpenAI, llama.cpp / LM Studio)
- **Tools** — native Rust functions or external [MCP](https://modelcontextprotocol.io/) servers
- **Workflows** — sequential, conditional, transform, and nested sub-workflow steps
- **Streaming** — token-by-token LLM output via channels
- **Events** — unified `scope × type × status` event stream for full observability
- **Context management** — pluggable history pruning (token budget, sliding window, summarization)
- **Tool loop prevention** — detects and short-circuits repeat tool calls
- **Config** — load runtime config from YAML or TOML

## Install

```toml
[dependencies]
agent-runtime = "0.4"
tokio = { version = "1", features = ["full"] }
```

## Quick start

### Agent + llama.cpp / LM Studio

```rust
use agent_runtime::llm::LlamaClient;
use agent_runtime::types::AgentInput;
use agent_runtime::{Agent, AgentConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(LlamaClient::new("http://localhost:8080", "llama"));

    let agent = Agent::new(
        AgentConfig::builder("assistant")
            .system_prompt("You are a helpful assistant.")
            .build(),
    )
    .with_client(client);

    let output = agent
        .execute(&AgentInput::from_text("What is 42 * 137?"))
        .await?;

    println!("{}", output.data);
    Ok(())
}
```

### Agent with native tools

```rust
use agent_runtime::tools::{CalculatorTool, ToolRegistry};
use agent_runtime::{Agent, AgentConfig};
use std::sync::Arc;

let mut registry = ToolRegistry::new();
registry.register(CalculatorTool);

let agent = Agent::new(
    AgentConfig::builder("math-bot")
        .system_prompt("Use tools to compute answers.")
        .tools(Arc::new(registry))
        .build(),
)
.with_client(client);
```

### Workflow with multiple steps

```rust
use agent_runtime::workflow::steps::{AgentStep, TransformStep};
use agent_runtime::{Runtime, Workflow};

let workflow = Workflow::builder()
    .add_step(Box::new(AgentStep::new(researcher_config)))
    .add_step(Box::new(TransformStep::new(
        "summarize-prompt".into(),
        |data| serde_json::json!({ "text": format!("Summarize: {}", data) }),
    )))
    .add_step(Box::new(AgentStep::new(summarizer_config)))
    .build();

let runtime = Runtime::new();
let run = runtime.execute(workflow).await;
```

### Event streaming

```rust
use agent_runtime::{EventScope, EventType, Runtime};

let runtime = Runtime::new();
let mut rx = runtime.event_stream().subscribe();

tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match (event.scope, event.event_type) {
            (EventScope::LlmRequest, EventType::Progress) => {
                if let Some(chunk) = event.data.get("chunk").and_then(|c| c.as_str()) {
                    print!("{}", chunk);
                }
            }
            (EventScope::Tool, EventType::Completed) => {
                println!("✓ {}", event.component_id);
            }
            _ => {}
        }
    }
});

runtime.execute(workflow).await;
```

### MCP external tools

```rust
use agent_runtime::tools::McpClient;

let mcp = McpClient::new_stdio(
    "npx",
    vec!["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
).await?;

let tools = mcp.list_tools().await?;
```

### Configuration file

```yaml
# agent-runtime.yaml
llm:
  base_url: "http://localhost:8080"
  model: "llama"

agents:
  - name: researcher
    system_prompt: "You are a research assistant."
    max_tool_iterations: 10
```

```rust
use agent_runtime::RuntimeConfig;

let config = RuntimeConfig::from_file("agent-runtime.yaml")?;
```

## Module layout

```
src/
├── agent/         Agent + AgentConfig + execution loop
├── config.rs      YAML/TOML configuration
├── context/       WorkflowContext + pruning strategies/
├── error.rs       Error types
├── event/         Event, EventStream, EventScope/Type/Status
├── llm/           LlmClient trait + provider/{llama, openai}
├── runtime/       Runtime + retry + timeout
├── tools/         Tool trait, registry, native, mcp, loop_detection, builtin
├── types.rs       AgentInput/Output, ToolResult, shared types
└── workflow/      Workflow + step + steps/{agent, transform, conditional, subworkflow}
```

## Event model

Every event has a **scope** (`Workflow`, `WorkflowStep`, `Agent`, `LlmRequest`, `Tool`, `System`),
a **type** (`Started`, `Progress`, `Completed`, `Failed`, `Canceled`), and a **status**.

Component IDs follow predictable formats: `workflow_name`, `workflow:step:N`, `agent_name`,
`agent:llm:N`, `tool_name:N`, `system:subsystem`.

## Documentation

- [`docs/`](docs/) — full guides for events, tools, workflows, MCP, configuration
- [`crates/agent-discourse/`](crates/agent-discourse/) — multi-agent demo

## Testing

```bash
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
