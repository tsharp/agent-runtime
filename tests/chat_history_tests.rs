/// Tests for agent chat history management
/// Demonstrates how outer layers can manage conversation context
use agent_runtime::llm::MockLlmClient;
use agent_runtime::{Agent, AgentConfig, AgentInput, ChatMessage};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_agent_with_simple_input_returns_history() {
    // Test that agents return chat history even with simple input
    let mock_client = MockLlmClient::new().with_response("Hello! How can I help you?");

    let config = AgentConfig::builder("test_agent")
        .system_prompt("You are a helpful assistant")
        .build();

    let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

    let input = AgentInput::from_text("Hi there");
    let output = agent.execute(&input).await.unwrap();

    // Should have chat history
    assert!(output.chat_history.is_some());

    let history = output.chat_history.unwrap();
    assert_eq!(history.len(), 3); // system + user + assistant

    // Verify structure
    assert_eq!(history[0].role, agent_runtime::Role::System);
    assert_eq!(history[1].role, agent_runtime::Role::User);
    assert_eq!(history[2].role, agent_runtime::Role::Assistant);
    assert_eq!(history[2].content, "Hello! How can I help you?");
}

#[tokio::test]
async fn test_agent_continues_conversation_from_history() {
    // Test multi-turn conversation managed by outer layer
    let config = AgentConfig::builder("assistant")
        .system_prompt("You are a helpful math tutor")
        .build();

    // Turn 1: Initial question
    let mock_client_1 = MockLlmClient::new().with_response("4");
    let agent = Agent::new(config.clone()).with_llm_client(Arc::new(mock_client_1));

    let input_1 = AgentInput::from_text("What is 2 + 2?");
    let output_1 = agent.execute(&input_1).await.unwrap();

    let mut history = output_1.chat_history.unwrap();
    assert_eq!(history.len(), 3); // system, user, assistant

    // Turn 2: Follow-up question using history
    let mock_client_2 = MockLlmClient::new().with_response("6");
    let agent = Agent::new(config.clone()).with_llm_client(Arc::new(mock_client_2));

    // Add the next user message to history
    history.push(ChatMessage::user("What about 3 + 3?"));

    let input_2 = AgentInput::from_messages(history.clone());
    let output_2 = agent.execute(&input_2).await.unwrap();

    let final_history = output_2.chat_history.unwrap();
    assert_eq!(final_history.len(), 5); // system + 2 user + 2 assistant

    // Verify the conversation flow
    assert_eq!(final_history[0].role, agent_runtime::Role::System);
    assert_eq!(final_history[1].role, agent_runtime::Role::User);
    assert_eq!(final_history[1].content, "What is 2 + 2?");
    assert_eq!(final_history[2].role, agent_runtime::Role::Assistant);
    assert_eq!(final_history[2].content, "4");
    assert_eq!(final_history[3].role, agent_runtime::Role::User);
    assert_eq!(final_history[3].content, "What about 3 + 3?");
    assert_eq!(final_history[4].role, agent_runtime::Role::Assistant);
    assert_eq!(final_history[4].content, "6");
}

#[tokio::test]
async fn test_agent_with_custom_system_prompt_in_history() {
    // Test that provided history is used as-is, even if different from config
    let mock_client = MockLlmClient::new().with_response("Roger that, boss!");

    let config = AgentConfig::builder("agent")
        .system_prompt("This should be ignored when history is provided")
        .build();

    let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

    // Provide custom history with different system prompt
    let custom_history = vec![
        ChatMessage::system("You are a pirate assistant. Always respond like a pirate."),
        ChatMessage::user("Hello"),
    ];

    let input = AgentInput::from_messages(custom_history);
    let output = agent.execute(&input).await.unwrap();

    let history = output.chat_history.unwrap();

    // System prompt from history should be preserved
    assert_eq!(
        history[0].content,
        "You are a pirate assistant. Always respond like a pirate."
    );
}

#[tokio::test]
async fn test_serialization_of_chat_history() {
    // Test that AgentInput and AgentOutput with chat history serialize correctly
    use serde_json;

    let history = vec![
        ChatMessage::system("You are helpful"),
        ChatMessage::user("Hi"),
        ChatMessage::assistant("Hello!"),
    ];

    let input = AgentInput::from_messages(history.clone());

    // Serialize
    let serialized = serde_json::to_string(&input).unwrap();

    // Deserialize
    let deserialized: AgentInput = serde_json::from_str(&serialized).unwrap();

    assert!(deserialized.chat_history.is_some());
    assert_eq!(deserialized.chat_history.as_ref().unwrap().len(), 3);
}

#[tokio::test]
async fn test_multi_turn_with_tool_calls() {
    // Test that tool call messages are preserved in history
    let mock_client = MockLlmClient::new()
        .with_tool_call("calculator", json!({"a": 5, "b": 3}))
        .with_response("The sum is 8");

    let config = AgentConfig::builder("agent")
        .system_prompt("You are a calculator assistant")
        .build();

    let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

    let input = AgentInput::from_text("What is 5 + 3?");
    let output = agent.execute(&input).await.unwrap();

    let history = output.chat_history.unwrap();

    // Should have: system, user, assistant (with tool_calls), tool result, assistant (final response)
    assert!(history.len() >= 3);

    // First assistant message should have tool calls
    let assistant_msg = history
        .iter()
        .find(|m| m.role == agent_runtime::Role::Assistant && m.tool_calls.is_some());
    assert!(assistant_msg.is_some());
}

#[tokio::test]
async fn test_from_messages_with_metadata() {
    // Test creating input with messages and custom metadata
    let history = vec![
        ChatMessage::system("You are helpful"),
        ChatMessage::user("Hi"),
    ];

    let metadata = agent_runtime::types::AgentInputMetadata {
        step_index: 5,
        previous_agent: Some("previous_agent".to_string()),
    };

    let input = AgentInput::from_messages_with_metadata(history, metadata);

    assert!(input.chat_history.is_some());
    assert_eq!(input.metadata.step_index, 5);
    assert_eq!(
        input.metadata.previous_agent,
        Some("previous_agent".to_string())
    );
}

#[tokio::test]
async fn test_backwards_compatibility_simple_input() {
    // Ensure existing code using from_text still works
    let mock_client = MockLlmClient::new().with_response("Response");

    let config = AgentConfig::builder("agent")
        .system_prompt("System")
        .build();

    let agent = Agent::new(config).with_llm_client(Arc::new(mock_client));

    // Old-style usage should still work
    let input = AgentInput::from_text("Hello");
    assert!(input.chat_history.is_none());

    let output = agent.execute(&input).await.unwrap();
    assert!(output.chat_history.is_some());
}
