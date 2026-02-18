# Sub-Workflow Context Sharing & Checkpointing

## Overview

This document describes how sub-workflows share conversation context with their parent workflows, and how to checkpoint/resume workflows with preserved conversation state.

## Sub-Workflow Context Sharing

### Design Philosophy

**Key Insight**: For end-to-end workflows, agents at all levels need access to the full conversation history. Sub-workflows are not isolated tasks - they're collaborative stages that build on previous work.

### How It Works

When a sub-workflow executes as a step in a parent workflow:
1. The parent's `WorkflowContext` is passed to the sub-workflow
2. Sub-workflow agents can see the full conversation history
3. Sub-workflow agents add their messages to the shared history
4. Parent workflow continues with the enriched history

```rust
// Parent workflow with context
let parent = Workflow::builder()
    .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
    .add_step(agent1)           // Adds to history
    .add_step(sub_workflow)     // Sees agent1's history, adds more
    .add_step(agent2)           // Sees everything
    .build();
```

### Benefits

- **Continuity**: Later agents see earlier findings
- **Synthesis**: Final agents can reference the entire process
- **Coherence**: One unified conversation, not fragmented contexts
- **Simplicity**: No manual history threading required

### Example: Research Pipeline

```rust
// Main workflow
let workflow = Workflow::builder()
    .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
    .add_step(researcher)       // "I found X, Y, Z"
    .add_step(sub_workflow)     // Detailed analysis sub-workflow
    .add_step(synthesizer)      // "Based on X, Y, Z and analysis..."
    .build();

// Sub-workflow (shares parent context)
let sub_workflow_builder = || {
    Workflow::builder()
        .add_step(detail_agent1)  // Sees researcher's findings
        .add_step(detail_agent2)  // Sees researcher + detail1
        .build()
};
```

**Conversation Flow:**
1. Researcher: "Found factors A, B, C"
2. Detail Agent 1: "Deep dive on A shows..."
3. Detail Agent 2: "Cross-ref with B and C..."
4. Synthesizer: "Based on all findings, recommend..."

All messages in one shared history!

## External Checkpointing

### Design Philosophy

Rather than build complex internal checkpointing, we expose `WorkflowContext` directly. External systems (databases, Redis, files) can serialize/restore it as needed.

### Basic Checkpointing

```rust
use agent_runtime::*;

// 1. Execute workflow
let workflow = Workflow::builder()
    .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
    .add_step(agent1)
    .build();

let ctx_ref = workflow.context().cloned().expect("Has context");
runtime.execute(workflow).await;

// 2. Checkpoint: Serialize context
let checkpoint = {
    let ctx = ctx_ref.read().unwrap();
    serde_json::to_string(&*ctx).unwrap()
};

// 3. Save externally (database, file, Redis, etc.)
database.save("checkpoint_id", checkpoint).await;
```

### Resuming from Checkpoint

```rust
// 1. Load checkpoint
let checkpoint_json = database.load("checkpoint_id").await;
let restored_context: WorkflowContext = 
    serde_json::from_str(&checkpoint_json).unwrap();

// 2. Resume workflow with restored context
let resumed_workflow = Workflow::builder()
    .with_restored_context(restored_context)
    .add_step(agent2)  // Continues where we left off
    .add_step(agent3)
    .build();

runtime.execute(resumed_workflow).await;
```

### What Gets Checkpointed

The `WorkflowContext` contains:
- **Complete conversation history** (all messages)
- **Token configuration** (max tokens, input/output ratio)
- **Workflow metadata** (ID, timestamps, step count)

```rust
#[derive(Serialize, Deserialize)]
pub struct WorkflowContext {
    pub chat_history: Vec<ChatMessage>,      // All conversation
    pub max_context_tokens: usize,           // e.g., 24_000
    pub input_output_ratio: f64,             // e.g., 3.0 (3:1)
    pub metadata: WorkflowMetadata,          // Tracking info
}
```

## Advanced Patterns

### Pattern 1: Multi-Stage with Checkpoints

```rust
// Stage 1: Research
let stage1 = Workflow::builder()
    .name("stage1_research")
    .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
    .add_step(researcher)
    .add_step(analyzer)
    .build();

let ctx1 = stage1.context().cloned().unwrap();
runtime.execute(stage1).await;

// Checkpoint after stage 1
let checkpoint1 = serialize_context(&ctx1);
save_checkpoint("stage1", checkpoint1).await;

// Stage 2: Detailed analysis (restored from checkpoint)
let checkpoint1 = load_checkpoint("stage1").await;
let restored = deserialize_context(&checkpoint1);

let stage2 = Workflow::builder()
    .name("stage2_analysis")
    .with_restored_context(restored)
    .add_step(SubWorkflowStep::new("details", detail_builder))
    .add_step(validator)
    .build();

runtime.execute(stage2).await;
```

### Pattern 2: Branching Workflows

```rust
// Execute main workflow
let main_workflow = create_workflow_with_context();
let ctx_ref = main_workflow.context().cloned().unwrap();
runtime.execute(main_workflow).await;

// Checkpoint
let checkpoint = serialize(&ctx_ref);

// Branch A: Continue in one direction
let restored_a = deserialize(&checkpoint);
let branch_a = Workflow::builder()
    .with_restored_context(restored_a)
    .add_step(specialist_a)
    .build();

// Branch B: Continue in another direction
let restored_b = deserialize(&checkpoint);
let branch_b = Workflow::builder()
    .with_restored_context(restored_b)
    .add_step(specialist_b)
    .build();

// Execute both branches from same checkpoint
runtime.execute(branch_a).await;
runtime.execute(branch_b).await;
```

### Pattern 3: Long-Running Workflows

```rust
// Workflow that can be paused/resumed over days
async fn long_running_research() {
    let checkpoint_db = CheckpointDatabase::new();
    
    // Day 1: Initial research
    let workflow1 = create_workflow();
    let ctx_ref = workflow1.context().cloned().unwrap();
    runtime.execute(workflow1).await;
    
    checkpoint_db.save("project_id", &ctx_ref).await;
    
    // Day 2: Continue from checkpoint
    let restored = checkpoint_db.load("project_id").await;
    let workflow2 = Workflow::builder()
        .with_restored_context(restored)
        .add_step(next_agent)
        .build();
    
    runtime.execute(workflow2).await;
}
```

## Database Integration Examples

### PostgreSQL

```rust
use sqlx::PgPool;

struct CheckpointStore {
    pool: PgPool,
}

impl CheckpointStore {
    async fn save(&self, id: &str, context: &WorkflowContext) -> Result<()> {
        let json = serde_json::to_string(context)?;
        
        sqlx::query!(
            "INSERT INTO checkpoints (id, context_json, created_at) 
             VALUES ($1, $2, NOW())
             ON CONFLICT (id) DO UPDATE SET context_json = $2",
            id,
            json
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    async fn load(&self, id: &str) -> Result<WorkflowContext> {
        let row = sqlx::query!(
            "SELECT context_json FROM checkpoints WHERE id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await?;
        
        let context = serde_json::from_str(&row.context_json)?;
        Ok(context)
    }
}
```

### Redis

```rust
use redis::AsyncCommands;

struct CheckpointCache {
    client: redis::Client,
}

impl CheckpointCache {
    async fn save(&self, id: &str, context: &WorkflowContext) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let json = serde_json::to_string(context)?;
        
        conn.set_ex(
            format!("checkpoint:{}", id),
            json,
            3600 * 24 * 7  // 7 days TTL
        ).await?;
        
        Ok(())
    }
    
    async fn load(&self, id: &str) -> Result<WorkflowContext> {
        let mut conn = self.client.get_async_connection().await?;
        let json: String = conn.get(format!("checkpoint:{}", id)).await?;
        let context = serde_json::from_str(&json)?;
        Ok(context)
    }
}
```

### File System

```rust
use std::fs;
use std::path::Path;

fn save_checkpoint(path: &Path, context: &WorkflowContext) -> Result<()> {
    let json = serde_json::to_string_pretty(context)?;
    fs::write(path, json)?;
    Ok(())
}

fn load_checkpoint(path: &Path) -> Result<WorkflowContext> {
    let json = fs::read_to_string(path)?;
    let context = serde_json::from_str(&json)?;
    Ok(context)
}
```

## Testing

The test suite includes:

### Sub-Workflow Tests (`tests/subworkflow_context_tests.rs`)
- ✅ Sub-workflows share parent context
- ✅ Nested sub-workflows (3 levels deep)
- ✅ Sub-workflows work without parent context (backward compat)

### Checkpoint Tests (`tests/checkpoint_tests.rs`)
- ✅ Checkpoint and restore workflow context
- ✅ External checkpoint management (database simulation)
- ✅ Token settings preserved across checkpoints
- ✅ WorkflowContext serialization/deserialization
- ✅ Restore without context manager (uses checkpoint settings)

## Best Practices

### 1. Always Reference Context Before Execution

```rust
// ✅ Good: Get reference before execute consumes workflow
let ctx_ref = workflow.context().cloned().expect("Has context");
runtime.execute(workflow).await;
let checkpoint = serialize(&ctx_ref);

// ❌ Bad: Can't access context after execution
runtime.execute(workflow).await;
let checkpoint = workflow.checkpoint_context(); // workflow was moved!
```

### 2. Checkpoint at Natural Boundaries

```rust
// ✅ Good: Checkpoint between logical stages
runtime.execute(research_workflow).await;
checkpoint("after_research");

runtime.execute(analysis_workflow).await;
checkpoint("after_analysis");

// ❌ Bad: Checkpointing mid-step is not supported
```

### 3. Handle Deserialization Errors

```rust
// ✅ Good: Handle potential errors
match serde_json::from_str::<WorkflowContext>(&json) {
    Ok(context) => resume_workflow(context).await,
    Err(e) => {
        log::error!("Failed to load checkpoint: {}", e);
        start_fresh_workflow().await
    }
}
```

### 4. Consider Checkpoint Size

```rust
// For very long workflows, consider:
// 1. Pruning old messages before checkpoint
// 2. Using SlidingWindowManager
// 3. Compressing checkpoint JSON

let context = ctx_ref.read().unwrap();
println!("Checkpoint will be ~{} KB", 
    serde_json::to_string(&*context).unwrap().len() / 1024
);
```

## Run the Demo

```bash
cargo run --bin advanced_workflow_demo
```

This demonstrates:
- Multi-stage workflow with chat history
- External checkpointing
- Workflow resumption
- Sub-workflow context sharing
- Complete conversation continuity

## Summary

**Sub-Workflow Context Sharing:**
- ✅ Sub-workflows automatically share parent context
- ✅ Enables full e2e workflows with complete history
- ✅ No manual context management needed
- ✅ Natural conversation flow across all levels

**External Checkpointing:**
- ✅ WorkflowContext is fully serializable
- ✅ You control storage (DB, Redis, files, etc.)
- ✅ Resume workflows from any checkpoint
- ✅ Simple, flexible, production-ready

This design provides maximum flexibility while keeping the implementation clean and maintainable.
