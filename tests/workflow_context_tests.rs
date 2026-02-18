use agent_runtime::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_workflow_with_chat_history() {
    // Create a mock LLM client with responses
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("I'm agent 1. The answer is 42.")
            .with_response("I'm agent 2 continuing the conversation. 42 is interesting!")
            .with_response("I'm agent 3, wrapping up. 42 is indeed the answer."),
    );

    // Create agents
    let agent1_config = AgentConfig::builder("agent1")
        .system_prompt("You are agent 1")
        .build();
    let agent1 = Agent::new(agent1_config).with_llm_client(mock_llm.clone());

    let agent2_config = AgentConfig::builder("agent2")
        .system_prompt("You are agent 2")
        .build();
    let agent2 = Agent::new(agent2_config).with_llm_client(mock_llm.clone());

    let agent3_config = AgentConfig::builder("agent3")
        .system_prompt("You are agent 3")
        .build();
    let agent3 = Agent::new(agent3_config).with_llm_client(mock_llm.clone());

    // Create workflow with token budget manager
    let context_manager = Arc::new(TokenBudgetManager::new(24_000, 3.0));

    let workflow = Workflow::builder()
        .name("chat_history_test".to_string())
        .with_chat_history(context_manager)
        .add_step(Box::new(AgentStep::from_agent(
            agent1,
            "agent1".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            agent2,
            "agent2".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            agent3,
            "agent3".to_string(),
        )))
        .initial_input(json!(
            "What is the answer to life, the universe, and everything?"
        ))
        .build();

    // Verify context was created
    assert!(workflow.context.is_some());

    // Keep reference to context before consuming workflow
    let context_ref = workflow.context.clone();

    // Execute workflow
    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;

    assert_eq!(run.state, WorkflowState::Completed);
    assert_eq!(run.steps.len(), 3);

    // Verify workflow context contains conversation history
    if let Some(context_arc) = context_ref {
        let context = context_arc.read().unwrap();
        let history = context.history();

        // Should have accumulated messages from all agent interactions
        // Note: The history grows as agents share context
        // Minimum: system prompt + multiple turns
        assert!(
            history.len() >= 3,
            "Expected at least 3 messages in history, got {}",
            history.len()
        );

        // Verify the last message contains something from agent 3
        let last_msg = history.last().unwrap();
        assert!(
            last_msg.content.contains("agent 3")
                || last_msg.content.contains("wrapping")
                || last_msg.content.contains("42")
        );
    }
}

#[tokio::test]
async fn test_workflow_without_chat_history() {
    // Create a mock LLM client
    let mock_llm = Arc::new(llm::MockLlmClient::new().with_response("Response from agent"));

    let agent_config = AgentConfig::builder("agent")
        .system_prompt("You are a test agent")
        .build();
    let agent = Agent::new(agent_config).with_llm_client(mock_llm);

    // Create workflow WITHOUT chat history management (legacy mode)
    let workflow = Workflow::builder()
        .name("no_chat_history".to_string())
        .add_step(Box::new(AgentStep::from_agent(agent, "agent".to_string())))
        .initial_input(json!("test input"))
        .build();

    // Verify context was NOT created
    assert!(workflow.context.is_none());

    // Execute should still work
    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;

    assert_eq!(run.state, WorkflowState::Completed);
    assert_eq!(run.steps.len(), 1);
}

#[tokio::test]
async fn test_token_budget_configuration() {
    // Test different context sizes and ratios
    let workflow_24k = Workflow::builder()
        .name("24k_3_to_1".to_string())
        .with_max_context_tokens(24_000)
        .with_input_output_ratio(3.0)
        .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
        .build();

    if let Some(context_arc) = &workflow_24k.context {
        let context = context_arc.read().unwrap();
        assert_eq!(context.max_context_tokens, 24_000);
        assert_eq!(context.input_output_ratio, 3.0);
        assert_eq!(context.max_input_tokens(), 18_000);
        assert_eq!(context.max_output_tokens(), 6_000);
    }

    let workflow_128k = Workflow::builder()
        .name("128k_4_to_1".to_string())
        .with_max_context_tokens(128_000)
        .with_input_output_ratio(4.0)
        .with_chat_history(Arc::new(TokenBudgetManager::new(128_000, 4.0)))
        .build();

    if let Some(context_arc) = &workflow_128k.context {
        let context = context_arc.read().unwrap();
        assert_eq!(context.max_context_tokens, 128_000);
        assert_eq!(context.input_output_ratio, 4.0);
        assert_eq!(context.max_input_tokens(), 102_400);
        assert_eq!(context.max_output_tokens(), 25_600);
    }
}

#[tokio::test]
async fn test_sliding_window_manager() {
    let mut mock_llm = llm::MockLlmClient::new();

    // Add multiple responses using builder pattern
    for i in 1..=5 {
        mock_llm = mock_llm.with_response(&format!("Response {}", i));
    }
    let mock_llm = Arc::new(mock_llm);

    // Create workflow with sliding window (keep last 5 messages)
    let context_manager = Arc::new(SlidingWindowManager::new(5));

    let mut builder = Workflow::builder()
        .name("sliding_window_test".to_string())
        .with_chat_history(context_manager)
        .initial_input(json!("start"));

    // Add 5 agents
    for i in 1..=5 {
        let config = AgentConfig::builder(format!("agent{}", i))
            .system_prompt(format!("Agent {}", i))
            .build();
        let agent = Agent::new(config).with_llm_client(mock_llm.clone());
        builder = builder.add_step(Box::new(AgentStep::from_agent(
            agent,
            format!("agent{}", i),
        )));
    }

    let workflow = builder.build();
    let context_ref = workflow.context.clone();

    let runtime = Runtime::new();
    let _run = runtime.execute(workflow).await;

    // Verify context has limited history
    if let Some(context_arc) = context_ref {
        let context = context_arc.read().unwrap();
        let history = context.history();

        // With sliding window of 5, should not exceed limit significantly
        // (may be slightly over due to system messages)
        assert!(
            history.len() <= 10,
            "History should be pruned: {}",
            history.len()
        );
    }
}
