#[cfg(test)]
mod tests {
    use crate::event::{ComponentStatus, EventScope, EventStream, EventType};
    use serde_json::json;

    #[test]
    fn test_event_stream_creation() {
        let stream = EventStream::new();
        assert_eq!(stream.len(), 0);
        assert!(stream.is_empty());
        assert_eq!(stream.current_offset(), 0);
    }

    #[tokio::test]
    async fn test_event_stream_append() {
        let stream = EventStream::new();

        let event = stream
            .workflow_started("wf_123", json!({"step_count": 3}))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(event.offset, 0);
        assert_eq!(event.scope, EventScope::Workflow);
        assert_eq!(event.event_type, EventType::Started);
        assert_eq!(event.component_id, "wf_123");
        assert_eq!(event.status, ComponentStatus::Running);
        assert_eq!(event.workflow_id, "wf_123");

        // Give async task time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(stream.len(), 1);
        assert_eq!(stream.current_offset(), 1);
    }

    #[tokio::test]
    async fn test_event_stream_multiple_events() {
        let stream = EventStream::new();

        stream.workflow_started("wf_123", json!({}));
        stream.step_started("wf_123", 0, json!({"step_name": "first"}));
        stream.step_completed("wf_123", 0, json!({}));

        // Give async tasks time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(stream.len(), 3);
        assert_eq!(stream.current_offset(), 3);
    }

    #[tokio::test]
    async fn test_event_stream_all() {
        let stream = EventStream::new();

        stream.workflow_started("wf_123", json!({}));
        stream.workflow_completed("wf_123", json!({}));

        // Give async tasks time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let all_events = stream.all();
        assert_eq!(all_events.len(), 2);
    }

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::Started;
        let json = serde_json::to_string(&event_type).unwrap();
        assert_eq!(json, "\"started\"");

        let scope = EventScope::LlmRequest;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"llm_request\"");

        let status = ComponentStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");
    }
}
