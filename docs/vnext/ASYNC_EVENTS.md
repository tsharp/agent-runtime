# Async Events Architecture (Future Enhancement)

## Overview

This document describes the ideal long-term architecture for the event system in agent-runtime. While the current implementation (v0.1) uses spawned tasks with `RwLock`, the optimal design would use fully async primitives with a dedicated background worker.

## Current Implementation (v0.1)

### Approach
- Events are emitted by spawning async tasks via `tokio::spawn()`
- `EventStream::append()` returns a `JoinHandle<Event>` that callers can await if needed
- Most callers ignore the return value (fire-and-forget)
- Internal state uses `RwLock<Vec<Event>>` (synchronous locking)
- Broadcasting uses `tokio::sync::broadcast` channel

### Benefits
- ✅ Non-blocking: agent execution never waits for event emission
- ✅ Backward compatible: callers don't need to change
- ✅ Minimal changes: single file modified (src/event.rs)
- ✅ Optional awaiting: callers can await if they need the Event object

### Limitations
- ⚠️ `RwLock` still blocks briefly (microseconds) during lock acquisition
- ⚠️ Each event spawns a new task (small overhead)
- ⚠️ No guaranteed ordering under heavy contention
- ⚠️ Broadcast channel can drop events if full (silent failure)

## Recommended Future Architecture

### Design: Async Channel + Background Worker

```rust
use tokio::sync::mpsc;

pub struct EventStream {
    sender: mpsc::UnboundedSender<EventCommand>,
    // Read-only state for queries (uses Arc<RwLock> or dashmap)
    events: Arc<RwLock<Vec<Event>>>,
    broadcaster: Arc<broadcast::Sender<Event>>,
}

enum EventCommand {
    Append {
        event_type: EventType,
        workflow_id: String,
        parent_workflow_id: Option<String>,
        data: Value,
        response: Option<oneshot::Sender<Event>>,
    },
}

impl EventStream {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let events = Arc::new(RwLock::new(Vec::new()));
        let (broadcaster, _) = broadcast::channel(1000);
        
        // Spawn single background worker
        let worker = EventWorker {
            receiver: rx,
            events: events.clone(),
            broadcaster: broadcaster.clone(),
            offset: 0,
        };
        tokio::spawn(worker.run());
        
        Self {
            sender: tx,
            events,
            broadcaster: Arc::new(broadcaster),
        }
    }
    
    pub async fn append(
        &self,
        event_type: EventType,
        workflow_id: String,
        data: Value,
    ) -> Event {
        let (tx, rx) = oneshot::channel();
        self.sender.send(EventCommand::Append {
            event_type,
            workflow_id,
            parent_workflow_id: None,
            data,
            response: Some(tx),
        }).unwrap();
        rx.await.unwrap()
    }
    
    pub fn append_fire_and_forget(
        &self,
        event_type: EventType,
        workflow_id: String,
        data: Value,
    ) {
        let _ = self.sender.send(EventCommand::Append {
            event_type,
            workflow_id,
            parent_workflow_id: None,
            data,
            response: None,
        });
    }
}

struct EventWorker {
    receiver: mpsc::UnboundedReceiver<EventCommand>,
    events: Arc<RwLock<Vec<Event>>>,
    broadcaster: broadcast::Sender<Event>,
    offset: usize,
}

impl EventWorker {
    async fn run(mut self) {
        while let Some(cmd) = self.receiver.recv().await {
            match cmd {
                EventCommand::Append {
                    event_type,
                    workflow_id,
                    parent_workflow_id,
                    data,
                    response,
                } => {
                    let event = Event {
                        offset: self.offset,
                        event_type,
                        workflow_id,
                        parent_workflow_id,
                        timestamp: Utc::now(),
                        data,
                    };
                    
                    // Single writer, no contention
                    self.events.write().unwrap().push(event.clone());
                    self.offset += 1;
                    
                    // Best-effort broadcast (log errors)
                    if let Err(e) = self.broadcaster.send(event.clone()) {
                        eprintln!("Event broadcast failed: {}", e);
                    }
                    
                    // Send response if requested
                    if let Some(tx) = response {
                        let _ = tx.send(event);
                    }
                }
            }
        }
    }
}
```

### Benefits

1. **Perfect Ordering**: Single writer guarantees sequential event ordering
2. **Zero Blocking**: Main execution path only sends to unbounded channel (instant)
3. **Batching Potential**: Worker can batch writes and broadcasts
4. **Error Handling**: Centralized error logging and recovery
5. **Resource Efficiency**: Single background task vs. one task per event
6. **Backpressure**: Can switch to bounded channel if needed
7. **Graceful Shutdown**: Worker can flush pending events on drop

### Performance Characteristics

| Operation | Current (v0.1) | Future Architecture |
|-----------|----------------|---------------------|
| Event emission | ~1-5μs (RwLock) + spawn overhead | ~0.1-0.5μs (channel send) |
| Ordering guarantee | Best-effort | Strong (sequential) |
| Memory overhead | Task per event | Single task |
| Broadcast reliability | Silent drops | Logged failures |
| Contention behavior | Lock contention | Queue buildup |

### Migration Path

#### Phase 1: Deprecation (v0.2)
- Keep existing `append()` method
- Add new `append_async()` method with async signature
- Add `append_sync()` for fire-and-forget
- Mark `append()` as deprecated with migration guide

#### Phase 2: Parallel Implementation (v0.3)
- Implement background worker behind feature flag `async-events`
- Run both implementations in parallel for validation
- Benchmark and compare behavior

#### Phase 3: Full Migration (v1.0)
- Remove old `RwLock`-based implementation
- Make `append_async()` the default
- Breaking change: `append()` becomes async

### Alternative Designs Considered

#### Option A: Async RwLock (tokio::sync::RwLock)
```rust
events: Arc<tokio::sync::RwLock<Vec<Event>>>
```
**Pros**: Drop-in replacement  
**Cons**: Still contention, less efficient than channel

#### Option B: Lock-Free Data Structures
```rust
events: Arc<SegQueue<Event>> // crossbeam
```
**Pros**: True zero-lock contention  
**Cons**: Complex, no ordering guarantees, harder to query

#### Option C: Actor Pattern (Actix)
**Pros**: Well-tested, production-ready  
**Cons**: Heavy dependency, overkill for simple event stream

**Decision**: Background worker provides best balance of simplicity, performance, and maintainability.

## Implementation Checklist

When implementing this design:

- [ ] Create `EventCommand` enum for worker messages
- [ ] Implement `EventWorker` with background task
- [ ] Add `append_async()` method returning `Event`
- [ ] Add `append_fire_and_forget()` for no-wait usage
- [ ] Deprecate current `append()` with timeline
- [ ] Add feature flag `async-events` for gradual rollout
- [ ] Benchmark performance vs. current implementation
- [ ] Update documentation with migration guide
- [ ] Add tests for ordering guarantees
- [ ] Add tests for graceful shutdown
- [ ] Consider adding event batching (flush every N events or T seconds)
- [ ] Add telemetry: queue depth, processing latency, drop count

## Related Future Work

- **Event Persistence**: Save events to disk/database for replay
- **Event Filtering**: Allow subscribers to filter by event type
- **Event Compression**: Compact old events to save memory
- **Distributed Events**: Synchronize events across multiple processes
- **Event Replay**: Reset stream to previous offset for debugging

## References

- [Tokio MPSC Channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)
- [Actor Pattern in Rust](https://ryhl.io/blog/actors-with-tokio/)
- [Lock-Free Programming](https://preshing.com/20120612/an-introduction-to-lock-free-programming/)

---

**Status**: Proposed for v1.0  
**Author**: Generated from v0.1 async events implementation  
**Last Updated**: 2024
