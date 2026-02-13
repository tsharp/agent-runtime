# agent-runtime

A Rust implementation of the Model Context Protocol (MCP) for composing AI agents, tools, and workflows. The crate provides a small runtime, event stream, and reusable step types so you can chain agents together or embed them inside other systems.

## Features
- Workflow builder and runtime with step-by-step execution history
- Event stream for live progress (streamed LLM chunks, failures, completion)
- Pluggable LLM clients (OpenAI and llama.cpp HTTP endpoints included)
- Ready-made steps: agent execution, transforms, conditionals, and nested workflows
- Mermaid diagram export to visualize workflows

## Quick Start
1. Install Rust 1.75+ and clone the repo.
2. Run tests: `cargo test`
3. Try a demo workflow (adjust the LLM endpoint in the example):
   - `cargo run --bin workflow_demo`
   - Other samples: `hello_workflow`, `nested_workflow`, `multi_subscriber`, `step_types_demo`, `mermaid_viz`, `complex_viz`, `llm_demo`, `llama_demo`

## Architecture
- `runtime`: executes workflows and emits events
- `workflow`: builder + state for ordered steps
- `agent`: wraps an LLM-backed agent with prompts and tools
- `step` / `step_impls`: traits and common step implementations (agent, transform, conditional, sub-workflow)
- `llm`: provider-agnostic chat client trait with OpenAI and llama.cpp clients

## Mermaid Output
`Workflow::to_mermaid()` renders a graph of your workflow (see `mermaid_viz`/`complex_viz` bins for examples).

## License
Dual-licensed under MIT or Apache-2.0 at your option.
