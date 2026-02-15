use crate::types::{EventId, EventOffset, JsonValue, WorkflowId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

#[cfg(test)]
#[path = "event_test.rs"]
mod event_test;

/// Event types that can occur in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Workflow events
    WorkflowStarted,
    WorkflowStepStarted,
    WorkflowStepCompleted,
    WorkflowCompleted,
    WorkflowFailed,

    // Agent events
    AgentInitialized,
    AgentProcessing,
    AgentCompleted,
    AgentFailed,

    // LLM events
    AgentLlmRequestStarted,
    AgentLlmStreamChunk,
    AgentLlmRequestCompleted,
    AgentLlmRequestFailed,

    // Tool events
    ToolCallStarted,
    ToolCallCompleted,
    ToolCallFailed,
    AgentToolLoopDetected,

    // System events
    SystemError,
    StateSaved,
}

/// An immutable event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub offset: EventOffset,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub workflow_id: WorkflowId,

    /// Optional parent workflow ID for nested workflows
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_workflow_id: Option<WorkflowId>,

    pub data: JsonValue,
}

impl Event {
    pub fn new(
        offset: EventOffset,
        event_type: EventType,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> Self {
        Self {
            id: format!("evt_{}", uuid::Uuid::new_v4()),
            offset,
            timestamp: Utc::now(),
            event_type,
            workflow_id,
            parent_workflow_id: None,
            data,
        }
    }

    pub fn with_parent(
        offset: EventOffset,
        event_type: EventType,
        workflow_id: WorkflowId,
        parent_workflow_id: Option<WorkflowId>,
        data: JsonValue,
    ) -> Self {
        Self {
            id: format!("evt_{}", uuid::Uuid::new_v4()),
            offset,
            timestamp: Utc::now(),
            event_type,
            workflow_id,
            parent_workflow_id,
            data,
        }
    }
}

/// Event stream with broadcast capability for real-time subscribers
pub struct EventStream {
    /// Broadcast sender for real-time event streaming
    sender: broadcast::Sender<Event>,

    /// Historical events for replay (thread-safe)
    history: Arc<RwLock<Vec<Event>>>,

    /// Next offset to assign
    next_offset: Arc<RwLock<EventOffset>>,
}

impl EventStream {
    /// Create a new event stream with specified channel capacity
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create event stream with custom channel capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);

        Self {
            sender,
            history: Arc::new(RwLock::new(Vec::new())),
            next_offset: Arc::new(RwLock::new(0)),
        }
    }

    /// Append a new event and broadcast to all subscribers
    ///
    /// Events are emitted asynchronously in a spawned task to avoid blocking
    /// agent execution. Returns a JoinHandle that can be awaited if the caller
    /// needs to ensure the event was processed or needs the Event object.
    ///
    /// # Examples
    /// ```no_run
    /// use agent_runtime::event::{EventStream, EventType};
    /// use serde_json::json;
    ///
    /// # async fn example() {
    /// let stream = EventStream::new();
    ///
    /// // Fire and forget (most common)
    /// stream.append(EventType::AgentInitialized, "workflow_1".to_string(), json!({}));
    ///
    /// // Wait for event if needed
    /// let event = stream.append(EventType::AgentCompleted, "workflow_1".to_string(), json!({}))
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub fn append(
        &self,
        event_type: EventType,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Event> {
        self.append_with_parent(event_type, workflow_id, None, data)
    }

    /// Append event with optional parent workflow ID
    ///
    /// Events are emitted asynchronously to avoid blocking execution.
    /// Returns a JoinHandle that resolves to the created Event.
    pub fn append_with_parent(
        &self,
        event_type: EventType,
        workflow_id: WorkflowId,
        parent_workflow_id: Option<WorkflowId>,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Event> {
        let sender = self.sender.clone();
        let history = self.history.clone();
        let next_offset = self.next_offset.clone();

        // Spawn async task - never blocks the caller
        tokio::spawn(async move {
            // Get and increment offset atomically
            let offset = {
                let mut next_offset = next_offset.write().unwrap();
                let current = *next_offset;
                *next_offset += 1;
                current
            };

            let event =
                Event::with_parent(offset, event_type, workflow_id, parent_workflow_id, data);

            // Store in history
            history.write().unwrap().push(event.clone());

            // Broadcast to subscribers (ignore if no active receivers)
            let _ = sender.send(event.clone());

            event
        })
    }

    /// Subscribe to real-time event stream
    /// Returns a receiver that will get all future events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Get events from a specific offset (for replay)
    pub fn from_offset(&self, offset: EventOffset) -> Vec<Event> {
        let history = self.history.read().unwrap();
        history
            .iter()
            .filter(|e| e.offset >= offset)
            .cloned()
            .collect()
    }

    /// Get all events
    pub fn all(&self) -> Vec<Event> {
        self.history.read().unwrap().clone()
    }

    /// Get event count
    pub fn len(&self) -> usize {
        self.history.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.read().unwrap().is_empty()
    }

    /// Get the current offset (next event will have this offset)
    pub fn current_offset(&self) -> EventOffset {
        *self.next_offset.read().unwrap()
    }
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventStream {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            history: Arc::clone(&self.history),
            next_offset: Arc::clone(&self.next_offset),
        }
    }
}
