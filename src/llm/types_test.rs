#[cfg(test)]
mod tests {
    use crate::llm::types::{ChatMessage, ChatRequest, ChatResponse, Role, Usage};

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage::system("You are a helpful assistant");
        assert_eq!(msg.role, Role::System);
        assert_eq!(msg.content, "You are a helpful assistant");

        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");

        let msg = ChatMessage::assistant("Hi there!");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Hi there!");
    }

    #[test]
    fn test_chat_request_builder() {
        let messages = vec![
            ChatMessage::system("System prompt"),
            ChatMessage::user("User message"),
        ];

        let request = ChatRequest::new(messages.clone());
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, None);
        assert_eq!(request.max_tokens, None);

        let request = request
            .with_temperature(0.7)
            .with_max_tokens(100)
            .with_top_p(0.9);

        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.top_p, Some(0.9));
    }

    #[test]
    fn test_chat_response_creation() {
        let response = ChatResponse {
            content: "Test response".to_string(),
            model: "test-model".to_string(),
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some("stop".to_string()),
        };

        assert_eq!(response.content, "Test response");
        assert_eq!(response.model, "test-model");
        assert!(response.usage.is_some());

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::System;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"system\"");

        let role = Role::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let role = Role::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");
    }

    #[test]
    fn test_message_serialization() {
        let msg = ChatMessage::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"user\""));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_request_serialization() {
        let messages = vec![ChatMessage::user("Test")];
        let request = ChatRequest::new(messages)
            .with_temperature(0.5);

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("messages").is_some());
        assert_eq!(json.get("temperature").unwrap(), &0.5);
    }

    #[test]
    fn test_usage_calculation() {
        let usage = Usage {
            prompt_tokens: 15,
            completion_tokens: 25,
            total_tokens: 40,
        };

        assert_eq!(usage.total_tokens, 40);
        assert_eq!(usage.prompt_tokens + usage.completion_tokens, 40);
    }
}
