#[cfg(test)]
mod tests {
    use crate::event::{EventStream, EventType};
    use serde_json::json;

    #[test]
    fn test_event_stream_creation() {
        let stream = EventStream::new();
        assert_eq!(stream.len(), 0);
        assert!(stream.is_empty());
        assert_eq!(stream.current_offset(), 0);
    }

    #[test]
    fn test_event_stream_append() {
        let stream = EventStream::new();

        let event = stream.append(
            EventType::WorkflowStarted,
            "wf_123".to_string(),
            json!({"step_count": 3}),
        );

        assert_eq!(event.offset, 0);
        assert_eq!(event.event_type, EventType::WorkflowStarted);
        assert_eq!(event.workflow_id, "wf_123");
        assert_eq!(stream.len(), 1);
        assert_eq!(stream.current_offset(), 1);
    }

    #[test]
    fn test_event_stream_multiple_events() {
        let stream = EventStream::new();

        stream.append(EventType::WorkflowStarted, "wf_123".to_string(), json!({}));

        stream.append(
            EventType::WorkflowStepStarted,
            "wf_123".to_string(),
            json!({"step_index": 0}),
        );

        stream.append(
            EventType::WorkflowStepCompleted,
            "wf_123".to_string(),
            json!({"step_index": 0}),
        );

        assert_eq!(stream.len(), 3);
        assert_eq!(stream.current_offset(), 3);
    }

    #[test]
    fn test_event_stream_from_offset() {
        let stream = EventStream::new();

        stream.append(EventType::WorkflowStarted, "wf_123".to_string(), json!({}));
        stream.append(
            EventType::WorkflowStepStarted,
            "wf_123".to_string(),
            json!({}),
        );
        stream.append(
            EventType::WorkflowStepCompleted,
            "wf_123".to_string(),
            json!({}),
        );

        let events = stream.from_offset(1);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].offset, 1);
        assert_eq!(events[1].offset, 2);
    }

    #[test]
    fn test_event_stream_all() {
        let stream = EventStream::new();

        stream.append(EventType::WorkflowStarted, "wf_123".to_string(), json!({}));
        stream.append(
            EventType::WorkflowCompleted,
            "wf_123".to_string(),
            json!({}),
        );

        let all_events = stream.all();
        assert_eq!(all_events.len(), 2);
    }

    #[tokio::test]
    async fn test_event_stream_subscribe() {
        let stream = EventStream::new();
        let mut receiver = stream.subscribe();

        // Spawn a task to send an event
        let stream_clone = stream.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            stream_clone.append(EventType::WorkflowStarted, "wf_test".to_string(), json!({}));
        });

        // Receive the event
        let event =
            tokio::time::timeout(tokio::time::Duration::from_secs(1), receiver.recv()).await;

        assert!(event.is_ok());
        let event = event.unwrap().unwrap();
        assert_eq!(event.event_type, EventType::WorkflowStarted);
        assert_eq!(event.workflow_id, "wf_test");
    }

    #[test]
    fn test_event_with_parent() {
        let stream = EventStream::new();

        let event = stream.append_with_parent(
            EventType::WorkflowStarted,
            "wf_child".to_string(),
            Some("wf_parent".to_string()),
            json!({}),
        );

        assert_eq!(event.parent_workflow_id, Some("wf_parent".to_string()));
    }

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::WorkflowStarted;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"workflow_started\"");

        let event_type = EventType::AgentLlmStreamChunk;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"agent_llm_stream_chunk\"");
    }
}
