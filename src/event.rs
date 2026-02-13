use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::types::{EventId, EventOffset, WorkflowId, JsonValue};

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
    
    // Tool events
    ToolCallStarted,
    ToolCallCompleted,
    ToolCallFailed,
    
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
            data,
        }
    }
}

/// Event stream for observability
pub struct EventStream {
    events: Vec<Event>,
    next_offset: EventOffset,
}

impl EventStream {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_offset: 0,
        }
    }
    
    /// Append a new event
    pub fn append(&mut self, event_type: EventType, workflow_id: WorkflowId, data: JsonValue) -> Event {
        let event = Event::new(self.next_offset, event_type, workflow_id, data);
        self.next_offset += 1;
        self.events.push(event.clone());
        event
    }
    
    /// Get events from a specific offset
    pub fn from_offset(&self, offset: EventOffset) -> impl Iterator<Item = &Event> {
        self.events.iter().filter(move |e| e.offset >= offset)
    }
    
    /// Get all events
    pub fn all(&self) -> &[Event] {
        &self.events
    }
    
    /// Get event count
    pub fn len(&self) -> usize {
        self.events.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new()
    }
}
