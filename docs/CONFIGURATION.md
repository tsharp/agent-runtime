# Configuration Guide

The agent-runtime framework supports both programmatic configuration (builder pattern) and file-based configuration (YAML/TOML).

## Configuration Formats

### YAML Configuration

```yaml
# agent-runtime.yaml
agents:
  - name: researcher
    system_prompt: |
      You are a research assistant specialized in finding accurate information.
      Always cite your sources.
    max_iterations: 10
    tool_loop_detection:
      enabled: true
      custom_message: "I already called {tool_name} with these parameters. The result was: {previous_result}"
    
  - name: summarizer
    system_prompt: "You create concise summaries."
    max_iterations: 5
    tool_loop_detection:
      enabled: false

workflows:
  - name: research_and_summarize
    steps:
      - type: agent
        agent: researcher
      - type: transform
        description: "Extract key findings"
      - type: agent
        agent: summarizer
```

### TOML Configuration

```toml
# agent-runtime.toml
[[agents]]
name = "researcher"
system_prompt = """
You are a research assistant specialized in finding accurate information.
Always cite your sources.
"""
max_iterations = 10

[agents.tool_loop_detection]
enabled = true
custom_message = "I already called {tool_name} with these parameters. The result was: {previous_result}"

[[agents]]
name = "summarizer"
system_prompt = "You create concise summaries."
max_iterations = 5

[agents.tool_loop_detection]
enabled = false
```

## Loading Configuration

### Auto-detect Format
```rust
use agent_runtime::config::RuntimeConfig;

// Automatically detects format from extension (.yaml, .yml, .toml)
let config = RuntimeConfig::from_file("agent-runtime.yaml")?;
```

### Explicit Format
```rust
// Load YAML explicitly
let config = RuntimeConfig::from_yaml_file("config.yaml")?;

// Load TOML explicitly
let config = RuntimeConfig::from_toml_file("config.toml")?;
```

## Programmatic Configuration

### Agent Configuration
```rust
use agent_runtime::{AgentConfig, ToolLoopDetectionConfig};

let agent = AgentConfig::new("assistant")
    .with_system_prompt("You are a helpful assistant.")
    .with_max_iterations(10)
    .with_llm_client(Arc::new(llm_client))
    .with_tool_loop_detection(
        ToolLoopDetectionConfig::new()
            .with_custom_message("Stop calling {tool_name}!")
    )
    .build();
```

### Disable Loop Detection
```rust
let agent = AgentConfig::new("assistant")
    .disable_tool_loop_detection()
    .build();
```

## Tool Loop Detection Configuration

### Default Behavior
By default, tool loop detection is **enabled** with a helpful message:
```
I notice I'm calling {tool_name} again with the same parameters. 
The previous result was: {previous_result}
I should use this result instead of calling the tool again.
```

### Custom Messages
Messages support two placeholders:
- `{tool_name}` - Name of the tool being called
- `{previous_result}` - JSON result from the previous identical call

Example:
```rust
ToolLoopDetectionConfig::new()
    .with_custom_message(
        "The {tool_name} tool already returned: {previous_result}. Use this data."
    )
```

### Disabling Per-Agent
```rust
// Disable for specific agent
let agent = AgentConfig::new("explorer")
    .disable_tool_loop_detection()
    .build();
```

## Environment Variables

Environment variables can override configuration:
```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://api.openai.com/v1"
export LLAMA_BASE_URL="http://localhost:1234/v1"
```

```rust
let api_key = std::env::var("OPENAI_API_KEY")?;
let base_url = std::env::var("OPENAI_BASE_URL")?;

let llm = OpenAiClient::new(&base_url, &api_key);
```

## Configuration Best Practices

1. **Use files for static configuration** - System prompts, max iterations, workflow structure
2. **Use builder pattern for dynamic configuration** - Runtime-specific settings, API keys
3. **Enable loop detection by default** - Prevents token waste and infinite loops
4. **Custom messages for domain-specific agents** - Help the LLM understand context
5. **Keep API keys in environment variables** - Never commit secrets to config files

## Complete Example

**config.yaml:**
```yaml
agents:
  - name: data_fetcher
    system_prompt: "You fetch data using available tools."
    max_iterations: 8
    tool_loop_detection:
      enabled: true
      custom_message: "Data already fetched: {previous_result}"
      
  - name: analyzer
    system_prompt: "You analyze data patterns."
    max_iterations: 5
```

**main.rs:**
```rust
use agent_runtime::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = RuntimeConfig::from_file("config.yaml")?;
    
    // Create LLM client
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let llm = Arc::new(OpenAiClient::new(
        "https://api.openai.com/v1",
        &api_key
    ));
    
    // Build agents from config
    let fetcher = AgentConfig::from_config(&config.agents[0])
        .with_llm_client(llm.clone())
        .with_tools(fetch_tools())
        .build();
    
    let analyzer = AgentConfig::from_config(&config.agents[1])
        .with_llm_client(llm)
        .build();
    
    // Build workflow
    let workflow = Workflow::new("analysis")
        .add_step(AgentStep::new(fetcher))
        .add_step(AgentStep::new(analyzer))
        .build();
    
    // Execute
    let result = workflow.execute(
        AgentInput::from_text("Analyze sales data"),
        &mut event_rx
    ).await?;
    
    println!("Analysis: {}", result.data);
    Ok(())
}
```

## Validation

Configuration is validated at load time:
- Agent names must be unique
- System prompts cannot be empty
- Max iterations must be > 0
- Tool loop detection messages are validated for placeholders

```rust
match RuntimeConfig::from_file("config.yaml") {
    Ok(config) => println!("Config loaded successfully"),
    Err(e) => eprintln!("Invalid config: {}", e),
}
```
