use agent_runtime::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_subworkflow_shares_parent_context() {
    // Create mock LLM with responses for main and sub-workflow agents
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Main agent: Starting research")
            .with_response("Sub agent 1: Detailed analysis A")
            .with_response("Sub agent 2: Detailed analysis B")
            .with_response("Main agent: Final synthesis based on sub-workflow findings"),
    );

    // Create main workflow agent
    let main_agent1 = Agent::new(
        AgentConfig::builder("main_agent1")
            .system_prompt("You are the main research coordinator")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let main_agent2 = Agent::new(
        AgentConfig::builder("main_agent2")
            .system_prompt("You are the synthesis agent")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    // Create the sub-workflow (will be executed as a step)
    let mock_llm_clone = mock_llm.clone();
    let sub_workflow_builder = move || {
        let sub_agent1 = Agent::new(
            AgentConfig::builder("sub_agent1")
                .system_prompt("You are detailed analysis agent 1")
                .build(),
        )
        .with_llm_client(mock_llm_clone.clone());

        let sub_agent2 = Agent::new(
            AgentConfig::builder("sub_agent2")
                .system_prompt("You are detailed analysis agent 2")
                .build(),
        )
        .with_llm_client(mock_llm_clone.clone());

        Workflow::builder()
            .name("detail_analysis".to_string())
            .add_step(Box::new(AgentStep::from_agent(
                sub_agent1,
                "sub_agent1".to_string(),
            )))
            .add_step(Box::new(AgentStep::from_agent(
                sub_agent2,
                "sub_agent2".to_string(),
            )))
            .build()
    };

    // Create main workflow with chat history
    let context_manager = Arc::new(TokenBudgetManager::new(24_000, 3.0));

    let workflow = Workflow::builder()
        .name("main_research".to_string())
        .with_chat_history(context_manager)
        .with_max_context_tokens(24_000)
        .with_input_output_ratio(3.0)
        .add_step(Box::new(AgentStep::from_agent(
            main_agent1,
            "main_agent1".to_string(),
        )))
        .add_step(Box::new(SubWorkflowStep::new(
            "detail_analysis".to_string(),
            sub_workflow_builder,
        )))
        .add_step(Box::new(AgentStep::from_agent(
            main_agent2,
            "main_agent2".to_string(),
        )))
        .initial_input(json!("Research topic: AI in software development"))
        .build();

    // Keep reference to context
    let ctx_ref = workflow.context().cloned().expect("Should have context");

    // Execute workflow
    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;

    // Verify successful execution
    assert_eq!(run.state, WorkflowState::Completed);
    assert_eq!(run.steps.len(), 3);

    // Verify that sub-workflow agents added to shared history
    let ctx = ctx_ref.read().unwrap();
    let history = ctx.history();

    // Should have messages from:
    // 1. main_agent1
    // 2. sub_agent1 (from sub-workflow)
    // 3. sub_agent2 (from sub-workflow)
    // 4. main_agent2
    // All sharing the same context
    assert!(
        history.len() >= 4,
        "Expected at least 4 messages in shared history, got {}",
        history.len()
    );

    // Verify messages accumulated from all agents
    let history_text: Vec<String> = history.iter().map(|msg| msg.content.clone()).collect();
    let combined = history_text.join(" ");

    // Should contain content from all stages
    assert!(
        combined.contains("Main agent") || combined.contains("Starting"),
        "Main agent 1 response not found"
    );
    assert!(
        combined.contains("Sub agent") || combined.contains("Detailed"),
        "Sub-workflow agent responses not found"
    );
    assert!(
        combined.contains("synthesis") || combined.contains("Final"),
        "Main agent 2 response not found"
    );
}

#[tokio::test]
async fn test_nested_subworkflows_share_context() {
    // Test that deeply nested sub-workflows all share the same context
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Level 0 agent")
            .with_response("Level 1 agent")
            .with_response("Level 2 agent")
            .with_response("Back to level 0"),
    );

    // Level 2 workflow (deepest)
    let mock_llm_l2 = mock_llm.clone();
    let level2_builder = move || {
        let level2_agent = Agent::new(
            AgentConfig::builder("level2")
                .system_prompt("Level 2 agent")
                .build(),
        )
        .with_llm_client(mock_llm_l2.clone());

        Workflow::builder()
            .name("level2".to_string())
            .add_step(Box::new(AgentStep::from_agent(
                level2_agent,
                "level2".to_string(),
            )))
            .build()
    };

    // Level 1 workflow (includes level 2 as sub-workflow)
    let mock_llm_l1 = mock_llm.clone();
    let level1_builder = move || {
        let level1_agent = Agent::new(
            AgentConfig::builder("level1")
                .system_prompt("Level 1 agent")
                .build(),
        )
        .with_llm_client(mock_llm_l1.clone());

        Workflow::builder()
            .name("level1".to_string())
            .add_step(Box::new(AgentStep::from_agent(
                level1_agent,
                "level1".to_string(),
            )))
            .add_step(Box::new(SubWorkflowStep::new(
                "level2".to_string(),
                level2_builder.clone(),
            )))
            .build()
    };

    // Level 0 workflow (main)
    let level0_agent1 = Agent::new(
        AgentConfig::builder("level0_start")
            .system_prompt("Level 0 start agent")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let level0_agent2 = Agent::new(
        AgentConfig::builder("level0_end")
            .system_prompt("Level 0 end agent")
            .build(),
    )
    .with_llm_client(mock_llm);

    let workflow = Workflow::builder()
        .name("level0".to_string())
        .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
        .with_max_context_tokens(24_000)
        .with_input_output_ratio(3.0)
        .add_step(Box::new(AgentStep::from_agent(
            level0_agent1,
            "level0_start".to_string(),
        )))
        .add_step(Box::new(SubWorkflowStep::new(
            "level1".to_string(),
            level1_builder,
        )))
        .add_step(Box::new(AgentStep::from_agent(
            level0_agent2,
            "level0_end".to_string(),
        )))
        .initial_input(json!("Deep nesting test"))
        .build();

    let ctx_ref = workflow.context().cloned().expect("Should have context");

    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;

    assert_eq!(run.state, WorkflowState::Completed);

    // Verify all levels shared the same context
    let ctx = ctx_ref.read().unwrap();
    let history = ctx.history();

    // Should have messages from all levels
    assert!(
        history.len() >= 4,
        "Expected messages from all nesting levels, got {}",
        history.len()
    );
}

#[tokio::test]
async fn test_subworkflow_without_parent_context() {
    // Test that sub-workflows work even when parent has no context
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Main agent")
            .with_response("Sub agent"),
    );

    let main_agent = Agent::new(
        AgentConfig::builder("main")
            .system_prompt("Main agent")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let sub_mock = mock_llm.clone();
    let sub_builder = move || {
        let sub_agent = Agent::new(
            AgentConfig::builder("sub")
                .system_prompt("Sub agent")
                .build(),
        )
        .with_llm_client(sub_mock.clone());

        Workflow::builder()
            .name("sub".to_string())
            .add_step(Box::new(AgentStep::from_agent(
                sub_agent,
                "sub".to_string(),
            )))
            .build()
    };

    // Create workflow WITHOUT chat history (legacy mode)
    let workflow = Workflow::builder()
        .name("no_context_parent".to_string())
        .add_step(Box::new(AgentStep::from_agent(
            main_agent,
            "main".to_string(),
        )))
        .add_step(Box::new(SubWorkflowStep::new(
            "sub".to_string(),
            sub_builder,
        )))
        .initial_input(json!("Test"))
        .build();

    // Verify no context
    assert!(workflow.context().is_none());

    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;

    // Should still work without context
    assert_eq!(run.state, WorkflowState::Completed);
    assert_eq!(run.steps.len(), 2);
}
