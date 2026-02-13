# Agent Workflow Runtime System - Specification

**Version:** 0.1.0  
**Last Updated:** 2026-02-12  
**Status:** Draft

## Table of Contents

1. [Overview](#overview)
2. [Core Concepts](#core-concepts)
3. [Architecture](#architecture)
4. [Components](#components)
   - [Workflow](#workflow)
   - [Agent](#agent)
   - [Tool System](#tool-system)
   - [Context Management](#context-management)
   - [State Management](#state-management)
   - [Event System](#event-system)
5. [Execution Model](#execution-model)
6. [Future Extensibility](#future-extensibility)
7. [Implementation Considerations](#implementation-considerations)

---

## Overview

The Agent Workflow Runtime System is a framework for orchestrating AI agents in structured workflows. It enables the composition of complex multi-agent systems where each agent operates with isolated context, dedicated tools, and clear input/output contracts.

### Goals

1. **Context Isolation**: Keep each agent's context clean and focused to minimize token usage and improve clarity
2. **Complete Observability**: Provide full visibility into workflow execution through comprehensive event streaming
3. **Immutable History**: Preserve complete execution history for debugging, auditing, and iterative refinement
4. **Resilient Communication**: Enable clients to stream events in real-time and replay from any checkpoint after disconnection
5. **Extensible Design**: Start with linear workflows while maintaining a path toward DAG-based execution patterns

### Use Cases

- **Multi-stage Processing**: Chain specialized agents for complex tasks (e.g., research → analysis → synthesis)
- **Iterative Refinement**: Support judge/critic patterns where agents review and refine outputs from other agents
- **Debuggable Execution**: Complete event history enables deep inspection of agent behavior and decision-making
- **Real-time Monitoring**: Clients can observe workflow progress through live event streams

## Core Concepts

### Workflow

A **Workflow** is a directed sequence of agent executions. It orchestrates the flow of data between agents and manages the overall execution lifecycle. In the initial implementation, workflows execute agents in linear sequence, with each agent's output feeding into the next agent's input.

### Agent

An **Agent** is an autonomous execution unit with:
- **System Prompt**: Defines the agent's role, behavior, and instructions
- **Tools**: A set of capabilities the agent can invoke to perform actions
- **Context**: An isolated execution environment independent of the workflow and other agents
- **Input/Output Contract**: Well-defined interface for receiving data and producing results

Agents are stateless and reusable - the same agent definition can be used in multiple workflows or multiple times within a workflow.

### Tool

A **Tool** is a discrete capability that an agent can invoke. Tools provide agents with the ability to:
- Interact with external systems (APIs, databases, file systems)
- Perform computations or transformations
- Access specialized functionality

Tools are registered with agents at configuration time and invoked during agent execution.

### Context

A **Context** is an isolated execution environment for an agent. Each agent execution receives a fresh context containing:
- The agent's system prompt
- Input data from the previous workflow step
- Access to configured tools
- No knowledge of other agents or workflow state

Context isolation ensures token efficiency and clarity of purpose for each agent.

### Event

An **Event** is an immutable record of a system activity. Events are emitted at every level:
- Workflow lifecycle (started, step completed, finished, failed)
- Agent execution (initialized, processing, completed, failed)
- Tool invocations (called, response received, error)
- State transitions and errors

Events are persisted with sequential offsets, enabling replay and audit capabilities.

## Architecture

### High-Level Structure

```
┌─────────────────────────────────────────────────────────┐
│                    Workflow Runtime                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Event Streaming Layer                 │  │
│  │         (HTTP Streaming + Offset Replay)          │  │
│  └───────────────────────────────────────────────────┘  │
│                          │                               │
│  ┌───────────────────────▼───────────────────────────┐  │
│  │              Workflow Orchestrator                 │  │
│  │          (Step Sequencing + State Flow)           │  │
│  └───────────────────────┬───────────────────────────┘  │
│                          │                               │
│         ┌────────────────┼────────────────┐             │
│         ▼                ▼                ▼             │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐        │
│  │ Agent 1  │────▶│ Agent 2  │────▶│ Agent 3  │        │
│  │          │     │          │     │          │        │
│  │ Context  │     │ Context  │     │ Context  │        │
│  │ + Tools  │     │ + Tools  │     │ + Tools  │        │
│  └──────────┘     └──────────┘     └──────────┘        │
│       │                │                │               │
│       └────────────────┴────────────────┘               │
│                          │                               │
│  ┌───────────────────────▼───────────────────────────┐  │
│  │           Execution History Store                  │  │
│  │      (Events + State + Agent Outputs)             │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Key Architectural Principles

1. **Unidirectional Data Flow**: Data flows sequentially from agent to agent, with each agent's output becoming the next agent's input
2. **Event-Driven Observability**: All actions emit events that bubble up through the system to the event streaming layer
3. **Persistent History**: All events and state transitions are stored, enabling replay and audit
4. **Isolation Boundaries**: Each agent operates in complete isolation with its own context and tool access

## Components

### Workflow

A Workflow defines a sequence of agent executions and manages their orchestration.

#### Structure

- **ID**: Unique identifier for the workflow instance
- **Definition**: Ordered list of agent configurations
- **Initial Input**: Starting data provided to the first agent
- **State**: Current execution state (pending, running, completed, failed)
- **Execution History**: Complete record of all events and agent outputs

#### Lifecycle

1. **Initialization**: Workflow is created with a definition and initial input
2. **Execution**: Agents are executed sequentially, each receiving the previous agent's output
3. **Completion**: All agents complete successfully, final output is produced
4. **Failure**: Any agent failure halts the workflow (error handling TBD)

#### Responsibilities

- Sequence agent executions in order
- Pass output from one agent as input to the next
- Emit workflow-level events (started, step_completed, finished, failed)
- Coordinate with event system for observability
- Maintain workflow state and execution history

### Agent

An Agent is a stateless execution unit that processes input using its system prompt and tools.

#### Configuration

- **Name**: Human-readable identifier
- **System Prompt**: Instructions defining the agent's role and behavior
- **Tool Registry**: Set of tools available to this agent
- **Model Configuration**: LLM model and parameters (temperature, max tokens, etc.)

#### Execution

When invoked within a workflow:

1. **Context Creation**: A fresh, isolated context is created
2. **Input Reception**: Receives input data from previous workflow step
3. **Processing**: Executes using system prompt, input, and available tools
4. **Output Production**: Produces structured output for next workflow step
5. **Event Emission**: Emits events for initialization, tool calls, and completion

#### Properties

- **Stateless**: No state persists between executions
- **Reusable**: Same agent definition can be used multiple times
- **Isolated**: No access to workflow state or other agents
- **Observable**: All actions produce events

### Tool System

Tools provide agents with capabilities to perform actions beyond text generation.

#### Tool Interface

Each tool implements:

- **Name**: Unique identifier
- **Description**: What the tool does (used by LLM for selection)
- **Input Schema**: Structured definition of required/optional parameters
- **Execute Function**: Async function that performs the tool's action
- **Output Schema**: Structure of the tool's return value

#### Tool Registration

Tools are registered with agents at configuration time:

```
Agent Config:
  - name: "researcher"
  - system_prompt: "You are a research assistant..."
  - tools: [web_search, document_reader, summarizer]
```

#### Tool Invocation

1. Agent requests tool execution with parameters
2. Runtime validates parameters against schema
3. Tool executes asynchronously
4. Events emitted: `tool_call_started`, `tool_call_completed`, `tool_call_failed`
5. Result returned to agent for continued processing

#### Tool Event Data

Tool events include:
- Tool name and parameters
- Execution duration
- Success/failure status
- Output or error details

### Context Management

Context isolation ensures each agent operates independently with minimal token usage.

#### Context Composition

Each agent execution receives a context containing:

- **System Prompt**: Agent's role and instructions
- **Input Data**: Output from previous workflow step (or initial input for first agent)
- **Tool Registry**: Available tools and their schemas
- **Conversation History**: Messages within this agent's execution only

#### Isolation Guarantees

- **No Workflow State Access**: Agent cannot see workflow definition or other agents
- **No Cross-Agent Communication**: Agents cannot directly interact with each other
- **No Persistent Memory**: Each execution starts fresh (stateless)
- **Independent Token Budget**: Context size is limited to this agent's execution

#### Context Lifecycle

1. **Creation**: New context created when agent begins execution
2. **Execution**: Agent processes input, potentially multiple LLM calls with tool usage
3. **Completion**: Final output extracted from context
4. **Archival**: Complete context saved to execution history
5. **Disposal**: Context resources released

#### Benefits

- **Token Efficiency**: Only relevant information in context
- **Clarity**: Agent purpose is focused and clear
- **Debuggability**: Complete context history available for inspection
- **Parallelization Ready**: Isolated contexts enable future parallel execution

### State Management

State flows sequentially through the workflow as data passes from agent to agent.

#### State Flow Pattern

```
Initial Input → Agent 1 → Output 1 → Agent 2 → Output 2 → Agent 3 → Final Output
```

#### State Structure

Each state transition includes:

- **Data**: The actual output from the previous agent (JSON or structured format)
- **Metadata**: Agent that produced it, timestamp, execution duration
- **Event Offset**: Reference to events emitted during this step

#### State Persistence

All state transitions are persisted to execution history:

- **Agent Inputs**: What each agent received
- **Agent Outputs**: What each agent produced
- **Intermediate States**: Complete state at each workflow step
- **Final State**: Ultimate workflow output

#### State Transformation

Agents can transform state in several ways:

- **Enrichment**: Add new information to existing data
- **Filtering**: Extract relevant subset of input
- **Restructuring**: Change data format or organization
- **Synthesis**: Combine multiple inputs into coherent output

The workflow runtime is agnostic to transformation logic - it simply passes agent output as-is to the next agent's input.

### Event System

The event system provides complete observability into workflow execution through persistent, replayable event streams.

#### Event Types

**Workflow Events**
- `workflow.started`: Workflow begins execution
- `workflow.step_started`: Agent step begins
- `workflow.step_completed`: Agent step finishes successfully
- `workflow.completed`: All agents complete, final output available
- `workflow.failed`: Workflow execution failed

**Agent Events**
- `agent.initialized`: Agent context created
- `agent.processing`: Agent is actively processing
- `agent.completed`: Agent finished successfully
- `agent.failed`: Agent execution failed

**Tool Events**
- `tool.call_started`: Tool invocation begins
- `tool.call_completed`: Tool returns successfully
- `tool.call_failed`: Tool execution error

**System Events**
- `system.error`: Unexpected system error
- `system.state_saved`: State persisted to history

#### Event Structure

```json
{
  "id": "evt_123456",
  "offset": 42,
  "timestamp": "2026-02-12T07:22:00Z",
  "type": "agent.completed",
  "workflow_id": "wf_789",
  "data": {
    "agent_name": "researcher",
    "duration_ms": 1234,
    "output_size": 5678
  }
}
```

#### Event Persistence

All events are stored with:
- **Sequential Offset**: Monotonically increasing integer for ordering
- **Immutability**: Events never modified after creation
- **Complete Data**: Full event payload preserved
- **Indexing**: Efficient querying by workflow_id, type, timestamp

#### Event Streaming

**HTTP Streaming Protocol**

Clients connect to event stream endpoint:
```
GET /workflows/{workflow_id}/events?offset={last_offset}
```

Server responds with:
- HTTP chunked transfer encoding
- Newline-delimited JSON events
- Real-time events as they occur
- Historical events if offset provided

**Reconnection & Replay**

When client reconnects:
1. Client provides last received offset
2. Server replays all events after that offset
3. Client catches up on missed events
4. Client receives real-time events going forward

This enables:
- Resume after network failure
- Complete event history replay (offset=0)
- Multiple clients with different positions
- Audit and debugging workflows

## Execution Model

### Linear Sequence Execution

In the initial implementation, workflows execute agents in strict linear order.

#### Execution Flow

1. **Workflow Start**
   - Workflow created with definition and initial input
   - Event: `workflow.started`
   - State initialized

2. **For Each Agent in Sequence**
   - **Step Start**: Event `workflow.step_started`
   - **Agent Initialize**: Create isolated context, Event `agent.initialized`
   - **Agent Execute**: 
     - Process input using system prompt
     - Invoke tools as needed (Events: `tool.call_*`)
     - Generate output
   - **Agent Complete**: Event `agent.completed`
   - **State Persist**: Save agent output to history
   - **Step Complete**: Event `workflow.step_completed`
   - **State Flow**: Agent output becomes next agent's input

3. **Workflow Complete**
   - All agents finished successfully
   - Event: `workflow.completed`
   - Final output available

#### Input/Output Contract

**Agent Input Structure**
```json
{
  "data": <output from previous agent or initial input>,
  "metadata": {
    "step_index": 0,
    "previous_agent": "researcher" // null for first agent
  }
}
```

**Agent Output Structure**
```json
{
  "data": <agent's produced output>,
  "metadata": {
    "agent_name": "researcher",
    "execution_time_ms": 1234,
    "tool_calls_count": 3
  }
}
```

#### Error Handling

**Agent Failure**
- Event: `agent.failed` with error details
- Event: `workflow.failed`
- Workflow halts, no subsequent agents execute
- Complete history preserved for debugging

**Tool Failure**
- Event: `tool.call_failed` with error
- Error returned to agent for handling
- Agent can retry, use fallback, or fail
- If agent fails, workflow fails

**Future Considerations**
- Retry policies for transient failures
- Fallback agents for failure recovery
- Partial success handling
- Circuit breaker patterns

#### Execution History

Complete execution record includes:
- All events in order with offsets
- All agent inputs and outputs
- All tool invocations and responses
- Execution timing and performance data
- Error information if failures occurred

This enables:
- Full replay of workflow execution
- Debugging agent behavior
- Performance analysis
- Audit trails
- Iterative refinement (judge pattern)

## Future Extensibility

The initial linear sequence design provides a foundation for more complex workflow patterns.

### DAG Workflow Support

**Design Considerations**

- **Node-Based Model**: Agents become nodes in a directed acyclic graph
- **Edge Definitions**: Explicit connections between agents define data flow
- **Multiple Inputs**: Agents can receive data from multiple predecessor nodes
- **Fan-out**: Single agent output can feed multiple downstream agents
- **Join Logic**: Define how multiple inputs merge at a single agent

**Architectural Changes Needed**

- Workflow definition format changes from list to graph structure
- State management handles multiple data streams
- Event model includes node dependencies and parallel execution
- Execution engine schedules based on dependency resolution

### Branching & Conditional Logic

**Conditional Paths**

- Agents can include routing logic in output
- Workflow engine interprets routing decisions
- Different agents execute based on conditions
- Events track which path was taken

**Example Use Case**
```
Input → Classifier Agent → [High Quality] → Detailed Processing
                        → [Low Quality] → Simple Processing
```

### Parallel Execution

**Parallelization Opportunities**

- Independent agent branches execute concurrently
- Isolated contexts enable safe parallelism
- Event system handles concurrent event emission
- Join nodes wait for all parallel branches

**Considerations**

- Resource management (concurrent LLM calls)
- Error handling across parallel branches
- Ordering guarantees for events
- Performance benefits vs complexity

### Judge/Refinement Pattern

**Iterative Improvement**

Current design supports judge patterns:

1. **Initial Workflow**: Agents produce output
2. **Judge Workflow**: New workflow evaluates outputs from execution history
3. **Refinement Workflow**: Targeted agents re-execute with feedback
4. **Iteration**: Repeat until judge approves or max iterations reached

**Future Enhancements**

- Built-in feedback loops within workflow
- Quality metrics and acceptance criteria
- Automatic retry with refined prompts
- Multi-judge consensus patterns

### Extension Points

The architecture includes natural extension points:

- **Tool System**: New tools easily added via registration
- **Event Listeners**: Hook into event stream for custom logic
- **State Transformers**: Custom state transformation between agents
- **Execution Strategies**: Pluggable execution engines (linear, DAG, custom)
- **Storage Backends**: Configurable persistence for events and history

## Implementation Considerations

### Rust Implementation

**Language Choice Benefits**

- **Type Safety**: Strong typing ensures correctness of agent configurations and data flow
- **Async Runtime**: Tokio provides efficient async/await for LLM calls and tool execution
- **Performance**: Fast execution with minimal overhead
- **Memory Safety**: No garbage collection pauses, predictable resource usage

**Key Dependencies**

- `tokio`: Async runtime for concurrent operations
- `serde/serde_json`: Event and state serialization
- `reqwest`: HTTP client for LLM API calls
- `actix-web`: HTTP server for event streaming endpoints
- `async-trait`: Async trait definitions for tools and agents

### Async & Concurrency

**Async Boundaries**

- Agent execution: `async fn execute(input: Input) -> Output`
- Tool invocation: `async fn call(params: Params) -> Result<Output>`
- Event emission: Non-blocking event append
- HTTP streaming: Async stream of events to clients

**Concurrency Model**

- Single workflow execution is sequential (initially)
- Multiple workflows can run concurrently
- Tool calls within agent can be concurrent (if agent supports)
- Event persistence is lock-free (append-only)

### Serialization & Persistence

**Event Storage**

- Append-only event log per workflow
- Format: Newline-delimited JSON (NDJSON)
- Indexing: In-memory offset → file position mapping
- Retention: Configurable (infinite, time-based, or size-based)

**State Storage**

- Each workflow step state persisted separately
- Format: JSON with schema versioning
- Location: Alongside event log or separate store
- Lifecycle: Persists with workflow execution history

**Storage Options**

- File system (simple, good for single-node)
- SQLite (embedded, queryable)
- PostgreSQL (production, distributed)
- S3/Object storage (archival, long-term retention)

### Event Streaming Implementation

**HTTP Streaming Details**

```rust
// Endpoint signature
GET /workflows/{workflow_id}/events?offset={offset}

// Response headers
Content-Type: application/x-ndjson
Transfer-Encoding: chunked
Cache-Control: no-cache

// Body: stream of newline-delimited JSON events
{"id":"evt_1","offset":0,"type":"workflow.started",...}\n
{"id":"evt_2","offset":1,"type":"agent.initialized",...}\n
{"id":"evt_3","offset":2,"type":"tool.call_started",...}\n
...
```

**Streaming Implementation**

- Use `actix-web::web::Bytes` stream
- Tokio channel between event system and HTTP handler
- Backpressure: Slow clients buffered with limits
- Timeout: Close connections inactive for N seconds
- Heartbeat: Periodic keep-alive comments

**Offset Management**

- Events assigned sequential offsets on creation
- Offsets are per-workflow (isolated sequences)
- Client tracks last received offset
- Server queries from offset+1 forward
- Offset 0 or omitted = replay entire history

### Performance Considerations

**Bottlenecks**

- LLM API latency (dominant factor)
- Event persistence (mitigated by append-only)
- HTTP streaming fanout (multiple clients)

**Optimizations**

- Connection pooling for LLM APIs
- Batch event writes (within latency budget)
- Event broadcast to multiple streaming clients
- Caching of agent definitions and tool schemas

### Error Handling

**Error Categories**

- **Transient**: Network timeouts, rate limits → retry
- **Invalid Input**: Schema validation failures → fail fast
- **Agent Failures**: LLM errors, tool failures → captured in events
- **System Failures**: Storage errors, OOM → critical, halt execution

**Error Propagation**

- All errors become events in stream
- Errors include full context for debugging
- Workflow state marked as failed
- Execution history preserved despite failures

### Testing Strategy

**Unit Tests**

- Individual tool implementations
- Event serialization/deserialization
- State transformation logic

**Integration Tests**

- Complete workflow execution
- Event streaming and replay
- Error handling paths

**End-to-End Tests**

- Multi-agent workflows
- Client reconnection scenarios
- Performance under load

---

## Summary

This specification defines an agent workflow runtime system with:

- **Isolated agent contexts** for token efficiency and clarity
- **Sequential execution** with clear input/output contracts
- **Complete event-driven observability** with HTTP streaming
- **Immutable execution history** supporting replay and audit
- **Extensible architecture** ready for DAG workflows and parallelism

The design prioritizes simplicity in initial implementation while maintaining a clear path toward advanced workflow patterns.
