use crate::types::{EventId, EventOffset, JsonValue, WorkflowId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

#[cfg(test)]
#[path = "event_test.rs"]
mod event_test;

/// Event scope - which component is emitting the event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventScope {
    Workflow,
    WorkflowStep,
    Agent,
    LlmRequest,
    Tool,
    System,
}

/// Event type - standard lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Started,
    Progress,
    Completed,
    Failed,
    Canceled,
}

/// Component status after event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
}

/// An immutable event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub offset: EventOffset,
    pub timestamp: DateTime<Utc>,

    /// Event scope (component type)
    pub scope: EventScope,

    /// Event type (lifecycle stage)
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// Component identifier (follows standardized format per scope)
    pub component_id: String,

    /// Current status of the component
    pub status: ComponentStatus,

    /// Workflow context
    pub workflow_id: WorkflowId,

    /// Optional parent workflow ID for nested workflows
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_workflow_id: Option<WorkflowId>,

    /// Optional human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Flexible payload for component-specific data
    pub data: JsonValue,
}

impl Event {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        offset: EventOffset,
        scope: EventScope,
        event_type: EventType,
        component_id: String,
        status: ComponentStatus,
        workflow_id: WorkflowId,
        message: Option<String>,
        data: JsonValue,
    ) -> Result<Self, String> {
        // Validate component_id format
        Self::validate_component_id(&scope, &component_id)?;

        Ok(Self {
            id: format!("evt_{}", uuid::Uuid::new_v4()),
            offset,
            timestamp: Utc::now(),
            scope,
            event_type,
            component_id,
            status,
            workflow_id,
            parent_workflow_id: None,
            message,
            data,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_parent(
        offset: EventOffset,
        scope: EventScope,
        event_type: EventType,
        component_id: String,
        status: ComponentStatus,
        workflow_id: WorkflowId,
        parent_workflow_id: Option<WorkflowId>,
        message: Option<String>,
        data: JsonValue,
    ) -> Result<Self, String> {
        // Validate component_id format
        Self::validate_component_id(&scope, &component_id)?;

        Ok(Self {
            id: format!("evt_{}", uuid::Uuid::new_v4()),
            offset,
            timestamp: Utc::now(),
            scope,
            event_type,
            component_id,
            status,
            workflow_id,
            parent_workflow_id,
            message,
            data,
        })
    }

    /// Validate component_id follows the required format for the scope
    fn validate_component_id(scope: &EventScope, component_id: &str) -> Result<(), String> {
        if component_id.is_empty() {
            return Err(format!("{:?} component_id cannot be empty", scope));
        }

        match scope {
            EventScope::Workflow => {
                // Simple name, no special format required
                Ok(())
            }
            EventScope::WorkflowStep => {
                // Must match: name:step:N
                let parts: Vec<&str> = component_id.split(':').collect();
                if parts.len() != 3 || parts[1] != "step" {
                    return Err(format!(
                        "WorkflowStep component_id must be 'workflow_name:step:N', got '{}'",
                        component_id
                    ));
                }
                // Validate N is a number
                if parts[2].parse::<usize>().is_err() {
                    return Err(format!(
                        "WorkflowStep index must be a number, got '{}'",
                        parts[2]
                    ));
                }
                Ok(())
            }
            EventScope::Agent => {
                // Simple name, no special format required
                Ok(())
            }
            EventScope::LlmRequest => {
                // Must match: agent_name:llm:N
                let parts: Vec<&str> = component_id.split(':').collect();
                if parts.len() != 3 || parts[1] != "llm" {
                    return Err(format!(
                        "LlmRequest component_id must be 'agent_name:llm:N', got '{}'",
                        component_id
                    ));
                }
                // Validate N is a number
                if parts[2].parse::<usize>().is_err() {
                    return Err(format!(
                        "LlmRequest iteration must be a number, got '{}'",
                        parts[2]
                    ));
                }
                Ok(())
            }
            EventScope::Tool => {
                // tool_name or tool_name:N
                // No strict format required, but validate not empty
                Ok(())
            }
            EventScope::System => {
                // Must start with 'system:'
                if !component_id.starts_with("system:") {
                    return Err(format!(
                        "System component_id must start with 'system:', got '{}'",
                        component_id
                    ));
                }
                Ok(())
            }
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
    /// use agent_runtime::event::{EventStream, EventScope, EventType, ComponentStatus};
    /// use serde_json::json;
    ///
    /// # async fn example() {
    /// let stream = EventStream::new();
    ///
    /// // Fire and forget (most common)
    /// stream.append(
    ///     EventScope::Agent,
    ///     EventType::Started,
    ///     "my_agent".to_string(),
    ///     ComponentStatus::Running,
    ///     "workflow_1".to_string(),
    ///     None,
    ///     json!({})
    /// );
    ///
    /// // Wait for event if needed
    /// let event = stream.append(
    ///     EventScope::Agent,
    ///     EventType::Completed,
    ///     "my_agent".to_string(),
    ///     ComponentStatus::Completed,
    ///     "workflow_1".to_string(),
    ///     Some("Agent completed successfully".to_string()),
    ///     json!({})
    /// ).await.unwrap();
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &self,
        scope: EventScope,
        event_type: EventType,
        component_id: String,
        status: ComponentStatus,
        workflow_id: WorkflowId,
        message: Option<String>,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append_with_parent(
            scope,
            event_type,
            component_id,
            status,
            workflow_id,
            None,
            message,
            data,
        )
    }

    /// Append event with optional parent workflow ID
    ///
    /// Events are emitted asynchronously to avoid blocking execution.
    /// Returns a JoinHandle that resolves to the created Event.
    #[allow(clippy::too_many_arguments)]
    pub fn append_with_parent(
        &self,
        scope: EventScope,
        event_type: EventType,
        component_id: String,
        status: ComponentStatus,
        workflow_id: WorkflowId,
        parent_workflow_id: Option<WorkflowId>,
        message: Option<String>,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
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

            let event = Event::with_parent(
                offset,
                scope,
                event_type,
                component_id,
                status,
                workflow_id,
                parent_workflow_id,
                message,
                data,
            )?;

            // Store in history
            history.write().unwrap().push(event.clone());

            // Broadcast to subscribers (ignore if no active receivers)
            let _ = sender.send(event.clone());

            Ok(event)
        })
    }

    // Helper methods for common event patterns

    /// Emit Agent::Started event
    pub fn agent_started(
        &self,
        agent_name: &str,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Agent,
            EventType::Started,
            agent_name.to_string(),
            ComponentStatus::Running,
            workflow_id,
            None,
            data,
        )
    }

    /// Emit Agent::Completed event
    pub fn agent_completed(
        &self,
        agent_name: &str,
        workflow_id: WorkflowId,
        message: Option<String>,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Agent,
            EventType::Completed,
            agent_name.to_string(),
            ComponentStatus::Completed,
            workflow_id,
            message,
            data,
        )
    }

    /// Emit Agent::Failed event
    pub fn agent_failed(
        &self,
        agent_name: &str,
        workflow_id: WorkflowId,
        error: &str,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Agent,
            EventType::Failed,
            agent_name.to_string(),
            ComponentStatus::Failed,
            workflow_id,
            Some(error.to_string()),
            data,
        )
    }

    /// Emit LlmRequest::Started event
    pub fn llm_started(
        &self,
        agent_name: &str,
        iteration: usize,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::LlmRequest,
            EventType::Started,
            format!("{}:llm:{}", agent_name, iteration),
            ComponentStatus::Running,
            workflow_id,
            None,
            data,
        )
    }

    /// Emit LlmRequest::Progress event (streaming chunk)
    pub fn llm_progress(
        &self,
        agent_name: &str,
        iteration: usize,
        workflow_id: WorkflowId,
        chunk: String,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::LlmRequest,
            EventType::Progress,
            format!("{}:llm:{}", agent_name, iteration),
            ComponentStatus::Running,
            workflow_id,
            None,
            serde_json::json!({ "chunk": chunk }),
        )
    }

    /// Emit LlmRequest::Completed event
    pub fn llm_completed(
        &self,
        agent_name: &str,
        iteration: usize,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::LlmRequest,
            EventType::Completed,
            format!("{}:llm:{}", agent_name, iteration),
            ComponentStatus::Completed,
            workflow_id,
            None,
            data,
        )
    }

    /// Emit LlmRequest::Failed event
    pub fn llm_failed(
        &self,
        agent_name: &str,
        iteration: usize,
        workflow_id: WorkflowId,
        error: &str,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::LlmRequest,
            EventType::Failed,
            format!("{}:llm:{}", agent_name, iteration),
            ComponentStatus::Failed,
            workflow_id,
            Some(error.to_string()),
            serde_json::json!({}),
        )
    }

    /// Emit Tool::Started event
    pub fn tool_started(
        &self,
        tool_name: &str,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Tool,
            EventType::Started,
            tool_name.to_string(),
            ComponentStatus::Running,
            workflow_id,
            None,
            data,
        )
    }

    /// Emit Tool::Progress event
    pub fn tool_progress(
        &self,
        tool_name: &str,
        workflow_id: WorkflowId,
        message: &str,
        percent: Option<u8>,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Tool,
            EventType::Progress,
            tool_name.to_string(),
            ComponentStatus::Running,
            workflow_id,
            Some(message.to_string()),
            serde_json::json!({ "percent": percent }),
        )
    }

    /// Emit Tool::Completed event
    pub fn tool_completed(
        &self,
        tool_name: &str,
        workflow_id: WorkflowId,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Tool,
            EventType::Completed,
            tool_name.to_string(),
            ComponentStatus::Completed,
            workflow_id,
            None,
            data,
        )
    }

    /// Emit Tool::Failed event
    pub fn tool_failed(
        &self,
        tool_name: &str,
        workflow_id: WorkflowId,
        error: &str,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Tool,
            EventType::Failed,
            tool_name.to_string(),
            ComponentStatus::Failed,
            workflow_id,
            Some(error.to_string()),
            data,
        )
    }

    /// Emit Workflow::Started event
    pub fn workflow_started(
        &self,
        workflow_name: &str,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Workflow,
            EventType::Started,
            workflow_name.to_string(),
            ComponentStatus::Running,
            workflow_name.to_string(),
            None,
            data,
        )
    }

    /// Emit Workflow::Completed event
    pub fn workflow_completed(
        &self,
        workflow_name: &str,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Workflow,
            EventType::Completed,
            workflow_name.to_string(),
            ComponentStatus::Completed,
            workflow_name.to_string(),
            None,
            data,
        )
    }

    /// Emit Workflow::Failed event
    pub fn workflow_failed(
        &self,
        workflow_name: &str,
        error: &str,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::Workflow,
            EventType::Failed,
            workflow_name.to_string(),
            ComponentStatus::Failed,
            workflow_name.to_string(),
            Some(error.to_string()),
            data,
        )
    }

    /// Emit WorkflowStep::Started event
    pub fn step_started(
        &self,
        workflow_name: &str,
        step_index: usize,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::WorkflowStep,
            EventType::Started,
            format!("{}:step:{}", workflow_name, step_index),
            ComponentStatus::Running,
            workflow_name.to_string(),
            None,
            data,
        )
    }

    /// Emit WorkflowStep::Completed event
    pub fn step_completed(
        &self,
        workflow_name: &str,
        step_index: usize,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::WorkflowStep,
            EventType::Completed,
            format!("{}:step:{}", workflow_name, step_index),
            ComponentStatus::Completed,
            workflow_name.to_string(),
            None,
            data,
        )
    }

    /// Emit WorkflowStep::Failed event
    pub fn step_failed(
        &self,
        workflow_name: &str,
        step_index: usize,
        error: &str,
        data: JsonValue,
    ) -> tokio::task::JoinHandle<Result<Event, String>> {
        self.append(
            EventScope::WorkflowStep,
            EventType::Failed,
            format!("{}:step:{}", workflow_name, step_index),
            ComponentStatus::Failed,
            workflow_name.to_string(),
            Some(error.to_string()),
            data,
        )
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
