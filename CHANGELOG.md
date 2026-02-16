# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2024

### ⚠️ BREAKING CHANGES

This release introduces a complete redesign of the event system with a unified, consistent pattern. **This is a breaking change** that requires migration for existing code.

#### Event System Redesign

**Old Pattern (v0.2.x)**:
- 19 specific event types (`WorkflowStarted`, `AgentProcessing`, `ToolCallStarted`, etc.)
- Inconsistent naming and lifecycle patterns
- Limited extensibility

**New Pattern (v0.3.0)**:
- Unified `Scope × Type × Status` model
- 6 event scopes: `Workflow`, `WorkflowStep`, `Agent`, `LlmRequest`, `Tool`, `System`
- 5 standard lifecycle types: `Started`, `Progress`, `Completed`, `Failed`, `Canceled`
- 5 component statuses: `Pending`, `Running`, `Completed`, `Failed`, `Canceled`

### Added

- **Event Scopes** (`EventScope` enum):
  - `Workflow` - Entire workflow execution
  - `WorkflowStep` - Individual workflow steps
  - `Agent` - Agent execution
  - `LlmRequest` - LLM/ChatClient requests (renamed from "Agent LLM")
  - `Tool` - Tool execution
  - `System` - System-level events

- **Unified Event Types** (`EventType` enum):
  - `Started` - Component begins execution
  - `Progress` - Component reports progress/streaming data
  - `Completed` - Component finished successfully
  - `Failed` - Component encountered error
  - **`Canceled`** - Component was canceled/interrupted (new!)

- **Component Status Tracking** (`ComponentStatus` enum):
  - Explicit status field on all events
  - Tracks component state after event

- **Component ID Validation**:
  - Enforced format standards per scope
  - `Workflow`: `workflow_name`
  - `WorkflowStep`: `workflow_name:step:N`
  - `Agent`: `agent_name`
  - `LlmRequest`: `agent_name:llm:N`
  - `Tool`: `tool_name`
  - `System`: `system:subsystem`

- **Helper Methods** on `EventStream`:
  - `agent_started()`, `agent_completed()`, `agent_failed()`
  - `llm_started()`, `llm_progress()`, `llm_completed()`, `llm_failed()`
  - `tool_started()`, `tool_progress()`, `tool_completed()`, `tool_failed()`
  - `workflow_started()`, `workflow_completed()`, `workflow_failed()`
  - `step_started()`, `step_completed()`, `step_failed()`

- **Event Fields**:
  - `component_id` - Standardized component identifier
  - `scope` - Event scope (which component type)
  - `status` - Current component status
  - `message` - Optional human-readable message

### Changed

- **Event struct**:
  - Added: `scope`, `event_type`, `component_id`, `status`, `message`
  - Renamed: `event_type` field is now the lifecycle stage, not the specific event
  
- **EventStream::append() signature**:
  ```rust
  // Old
  fn append(&self, event_type: EventType, workflow_id: String, data: JsonValue)
  
  // New
  fn append(
      &self, 
      scope: EventScope, 
      event_type: EventType,
      component_id: String,
      status: ComponentStatus,
      workflow_id: String,
      message: Option<String>,
      data: JsonValue
  )
  ```

- **Tool loop detection** now emits `System::Progress` events instead of `AgentToolLoopDetected`

- Version bumped from `0.1.0` → `0.3.0` (skip 0.2.0 due to breaking nature)

### Removed

- **Old EventType variants**:
  - `WorkflowStarted`, `WorkflowStepStarted`, `WorkflowStepCompleted`, `WorkflowCompleted`, `WorkflowFailed`
  - `AgentInitialized`, `AgentProcessing`, `AgentCompleted`, `AgentFailed`
  - `AgentLlmRequestStarted`, `AgentLlmStreamChunk`, `AgentLlmRequestCompleted`, `AgentLlmRequestFailed`
  - `ToolCallStarted`, `ToolCallCompleted`, `ToolCallFailed`, `AgentToolLoopDetected`
  - `SystemError`, `StateSaved`

### Migration Guide

See [docs/MIGRATION_0.2_TO_0.3.md](docs/MIGRATION_0.2_TO_0.3.md) for detailed migration instructions.

**Quick migration examples**:

```rust
// Old: Pattern matching on specific event types
match event.event_type {
    EventType::WorkflowStarted => { ... }
    EventType::ToolCallCompleted => { ... }
}

// New: Pattern matching on scope + type
match (event.scope, event.event_type) {
    (EventScope::Workflow, EventType::Started) => { ... }
    (EventScope::Tool, EventType::Completed) => { ... }
}

// Old: Manual event emission
stream.append(
    EventType::ToolCallStarted,
    "wf_1",
    json!({"tool": "my_tool"})
);

// New: Use helper methods
stream.tool_started(
    "my_tool",
    "wf_1".to_string(),
    json!({})
);
```

### Technical Details

- All events now emit asynchronously via spawned tasks (from v0.2.x)
- Event validation ensures component IDs follow standardized formats
- Helper methods provide type-safe, ergonomic API for common event patterns
- Raw `append()` method still available for custom event scenarios

### Tests

- 97 tests passing (63 lib + 7 chat_history + 12 error + 3 integration + 12 load)
- All clippy warnings resolved
- All doctests passing

---

## [0.2.0] - 2024 (Skipped)

Skipped to avoid confusion with pre-release versions.

## [0.1.0] - 2024

Initial release with basic agent runtime, workflow system, and event streaming.
