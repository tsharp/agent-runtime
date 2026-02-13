# Workflow Composition - Complete

## Overview

Workflows can now be nested - entire workflows can be embedded as steps within other workflows. This enables hierarchical composition, reusable workflow components, and complex multi-level processing pipelines.

## Architecture

### SubWorkflowStep

A step type that executes an entire workflow as part of another workflow:

```rust
pub struct SubWorkflowStep {
    name: String,
    workflow_builder: Box<dyn Fn() -> Workflow + Send + Sync>,
}
```

**Key Design:**
- Uses a builder function instead of storing a Workflow (avoids clone issues)
- Shares the parent's Runtime and EventStream
- Events emitted with `parent_workflow_id` for hierarchy tracking

### Event Hierarchy

Events from nested workflows include parent context:

```rust
pub struct Event {
    pub workflow_id: WorkflowId,
    pub parent_workflow_id: Option<WorkflowId>,  // NEW
    pub event_type: EventType,
    pub data: JsonValue,
    // ...
}
```

##Usage

### Basic Nested Workflow

```rust
// Define a reusable sub-workflow
let validation_workflow = || {
    Workflow::builder()
        .step(Box::new(TransformStep::new("validate", validate_fn)))
        .step(Box::new(TransformStep::new("sanitize", sanitize_fn)))
        .build()
};

// Use it in a main workflow
let main_workflow = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new(
        "validation_pipeline".to_string(),
        validation_workflow,
    )))
    .step(Box::new(AgentStep::new(processor)))
    .build();
```

### Multi-Level Nesting

Workflows can be nested multiple levels deep:

```rust
// Level 3: Data cleaning
let cleaning_workflow = || {
    Workflow::builder()
        .step(Box::new(TransformStep::new("trim", trim_fn)))
        .step(Box::new(TransformStep::new("normalize", normalize_fn)))
        .build()
};

// Level 2: Validation (includes cleaning)
let validation_workflow = || {
    Workflow::builder()
        .step(Box::new(SubWorkflowStep::new("clean", cleaning_workflow)))
        .step(Box::new(TransformStep::new("validate", validate_fn)))
        .build()
};

// Level 1: Main workflow (includes validation)
let main_workflow = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new("validate", validation_workflow)))
    .step(Box::new(AgentStep::new(processor)))
    .build();
```

## Benefits

### 1. Reusability

Define workflows once, use them everywhere:

```rust
// Common validation workflow
let email_validation = || {
    Workflow::builder()
        .step(Box::new(TransformStep::new("format_check", check_email_format)))
        .step(Box::new(TransformStep::new("domain_verify", verify_domain)))
        .build()
};

// Use in multiple contexts
let signup_workflow = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new("validate_email", email_validation)))
    .step(Box::new(AgentStep::new(create_account_agent)))
    .build();

let update_profile_workflow = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new("validate_email", email_validation)))
    .step(Box::new(AgentStep::new(update_agent)))
    .build();
```

### 2. Modularity

Break complex workflows into logical units:

```rust
let data_pipeline = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new("extract", extraction_workflow)))
    .step(Box::new(SubWorkflowStep::new("transform", transformation_workflow)))
    .step(Box::new(SubWorkflowStep::new("load", loading_workflow)))
    .build();
```

### 3. Testability

Test sub-workflows in isolation:

```rust
// Test validation workflow independently
let validation_wf = validation_workflow();
let result = runtime.execute(validation_wf).await;
assert_eq!(result.state, WorkflowState::Completed);

// Then use in larger workflow with confidence
```

### 4. Maintainability

Change sub-workflows without touching parent:

```rust
// Update validation logic in one place
let validation_workflow = || {
    Workflow::builder()
        .step(Box::new(TransformStep::new("validate_v2", new_validation)))
        // Changed implementation
        .build()
};

// All workflows using it automatically get the update
```

## Event Tracking

### Parent-Child Relationships

Events clearly show workflow hierarchy:

```
Main Workflow (wf_abc123)
  ├─ Event: WorkflowStarted
  ├─ Event: WorkflowStepStarted (sub-workflow step)
  │   └─ Sub-Workflow (wf_def456) [parent: wf_abc123]
  │       ├─ Event: WorkflowStarted [parent: wf_abc123]
  │       ├─ Event: WorkflowStepStarted [parent: wf_abc123]
  │       ├─ Event: WorkflowStepCompleted [parent: wf_abc123]
  │       └─ Event: WorkflowCompleted [parent: wf_abc123]
  ├─ Event: WorkflowStepCompleted
  └─ Event: WorkflowCompleted
```

### Filtering Events

Filter events by workflow or hierarchy level:

```rust
// All events from main workflow only
let main_events: Vec<_> = all_events
    .iter()
    .filter(|e| e.workflow_id == main_wf_id && e.parent_workflow_id.is_none())
    .collect();

// All events from a specific sub-workflow
let sub_events: Vec<_> = all_events
    .iter()
    .filter(|e| e.workflow_id == sub_wf_id)
    .collect();

// All events including children
let all_related: Vec<_> = all_events
    .iter()
    .filter(|e| {
        e.workflow_id == main_wf_id ||
        e.parent_workflow_id == Some(main_wf_id.clone())
    })
    .collect();
```

## Patterns

### ETL Pipeline

```rust
let etl_workflow = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new("extract", || {
        Workflow::builder()
            .step(Box::new(TransformStep::new("fetch", fetch_data)))
            .step(Box::new(TransformStep::new("parse", parse_data)))
            .build()
    })))
    .step(Box::new(SubWorkflowStep::new("transform", || {
        Workflow::builder()
            .step(Box::new(TransformStep::new("clean", clean_data)))
            .step(Box::new(AgentStep::new(enrichment_agent)))
            .build()
    })))
    .step(Box::new(SubWorkflowStep::new("load", || {
        Workflow::builder()
            .step(Box::new(TransformStep::new("validate", validate_data)))
            .step(Box::new(TransformStep::new("save", save_data)))
            .build()
    })))
    .build();
```

### Quality Assurance

```rust
let qa_workflow = Workflow::builder()
    .step(Box::new(AgentStep::new(generator_agent)))
    .step(Box::new(SubWorkflowStep::new("quality_check", || {
        Workflow::builder()
            .step(Box::new(AgentStep::new(critic_agent)))
            .step(Box::new(ConditionalStep::new(
                "check_quality",
                |data| data["score"].as_f64() > Some(0.8),
                Box::new(TransformStep::new("approve", approve_fn)),
                Box::new(TransformStep::new("reject", reject_fn)),
            )))
            .build()
    })))
    .build();
```

### Multi-Stage Processing

```rust
let document_workflow = Workflow::builder()
    .step(Box::new(SubWorkflowStep::new("preprocessing", preprocessing_wf)))
    .step(Box::new(SubWorkflowStep::new("analysis", analysis_wf)))
    .step(Box::new(SubWorkflowStep::new("summarization", summarization_wf)))
    .step(Box::new(SubWorkflowStep::new("postprocessing", postprocessing_wf)))
    .build();
```

## Technical Details

### Shared Event Stream

Sub-workflows use the parent's Runtime and EventStream:

```rust
// Inside Runtime::execute_with_parent
if step_type == StepType::SubWorkflow {
    // Share this runtime's event stream
    sub_step.execute_with_runtime(input, self).await
} else {
    step.execute(input).await
}
```

**Benefits:**
- All events in single stream
- Correct chronological ordering
- No event merging needed
- Simpler debugging

### Builder Pattern

Sub-workflows use builder functions instead of stored instances:

```rust
// Builder function - called each time
let workflow_builder = || {
    Workflow::builder()
        .step(...)
        .build()
};

SubWorkflowStep::new("name", workflow_builder)
```

**Why:**
- Avoids clone() issues with trait objects
- Each execution gets fresh workflow instance
- Simpler lifetimes
- More flexible (can capture different data each call)

## Example Output

```
=== Event Monitor ===
  [wf_abc12] WorkflowStarted
  [wf_abc12] WorkflowStepStarted
  [wf_def34] WorkflowStarted [parent: wf_abc12]
  [wf_def34] WorkflowStepStarted [parent: wf_abc12]
  [wf_def34] WorkflowStepCompleted [parent: wf_abc12]
  [wf_def34] WorkflowCompleted [parent: wf_abc12]
  [wf_abc12] WorkflowStepCompleted
  [wf_abc12] WorkflowCompleted

=== Event Hierarchy ===
Total workflows executed: 2
  Workflow wf_abc12: 4 events
  Workflow wf_def34 (child of wf_abc12): 4 events
```

## Testing

```bash
cargo run --bin nested_workflow
```

Shows:
- Main workflow with 3 steps
- 2 sub-workflows executed as steps
- Complete event hierarchy
- Parent-child relationships in events

## Next Steps

With workflow composition, you can now:
1. Build reusable workflow libraries
2. Create complex multi-stage pipelines
3. Test components in isolation
4. Compose workflows like LEGO blocks

Combined with the Step abstraction, you have:
- Agent steps (LLM processing)
- Transform steps (pure functions)
- Conditional steps (branching)
- **Sub-workflow steps (composition)**

This provides a complete toolkit for building sophisticated agent workflows!

---

## Summary

✅ Sub-workflows executed as steps  
✅ Events track parent-child relationships  
✅ Shared event stream across hierarchy  
✅ Reusable workflow components  
✅ Clean, modular architecture  
✅ Testable in isolation  
