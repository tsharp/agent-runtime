# Workflow Demo

Simple demonstration of the agent-runtime workflow system with real LLM integration.

## What It Does

Creates a 3-agent workflow that:
1. **Greeter** - Welcomes the user warmly
2. **Analyzer** - Analyzes the greeter's response
3. **Summarizer** - Summarizes the entire conversation

Each agent calls a real LLM (Qwen 3 30B via llama.cpp server) with its own system prompt and context.

## Prerequisites

**Start LM Studio or llama.cpp server on port 1234:**

### Option 1: LM Studio (Easiest)
1. Download and install [LM Studio](https://lmstudio.ai/)
2. Load a model (e.g., `qwen/qwen3-30b-a3b-2507`)
3. Start local server on port 1234 (default)

### Option 2: llama.cpp
```bash
# Download and build
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
make

# Download a model
wget https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf \
  -O models/qwen.gguf

# Start server
./server -m models/qwen.gguf --port 1234
```

## Run the Demo

```bash
cargo run --bin workflow_demo
```

## Expected Output

```
=== Workflow Demo ===

✓ LLM client configured (localhost:1234)

✓ Created 3 agents: greeter → analyzer → summarizer

✓ Workflow built with 3 sequential steps

Workflow Structure:
flowchart TD
    Start([Start])
    Start --> N0
    N0["greeter"]:::agentStyle
    ...

▶ Starting workflow execution...

============================================================
[Real LLM responses from each agent...]

============================================================

✅ Workflow execution complete!

Final Output:
{
  "response": "In summary, the conversation consisted of..."
}

Steps executed: 3
  Step 1: greeter (agent)
    Execution time: 1234ms
  Step 2: analyzer (agent)
    Execution time: 2156ms
  Step 3: summarizer (agent)
    Execution time: 987ms
```

## What's Happening

1. **Workflow Builder** - Constructs a linear 3-step workflow
2. **Agent Configuration** - Each agent has:
   - Unique name
   - Custom system prompt
   - Shared LLM client (Qwen 3 30B)
3. **Runtime Execution** - Sequential execution:
   - Step 1 gets initial input
   - Step 2 gets Step 1's output
   - Step 3 gets Step 2's output
4. **Real LLM Calls** - Each agent makes actual API calls to localhost:1234

## Code Structure

```rust
// Create LLM client
let llm_client = Arc::new(LlamaClient::new("http://localhost:1234", "qwen/qwen3-30b-a3b-2507"));

// Create agent with LLM
let agent = Agent::new(
    AgentConfig::builder("name")
        .system_prompt("You are a...")
        .build()
).with_llm_client(llm_client.clone());

// Build workflow
let workflow = Workflow::builder()
    .step(Box::new(AgentStep::from_agent(agent, "name".to_string())))
    .initial_input(serde_json::json!("Your input here"))
    .build();

// Execute
let runtime = Runtime::new();
let result = runtime.execute(workflow).await;
```

## Features Demonstrated

- ✅ Multi-agent workflows
- ✅ Real LLM integration (llama.cpp)
- ✅ Sequential step execution
- ✅ Context passing between agents
- ✅ System prompts per agent
- ✅ Execution time tracking
- ✅ Mermaid diagram generation

## Next Steps

Try modifying the demo to:
- Add more agents
- Change the system prompts
- Use different input
- Add conditional steps
- Add transform steps
- Use OpenAI instead of llama.cpp

See `docs/spec.md` for full workflow capabilities!
