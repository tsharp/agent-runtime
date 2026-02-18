use agent_runtime::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_checkpoint_and_restore_context() {
    // Create initial workflow with chat history
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("First agent response")
            .with_response("Second agent response after restore"),
    );

    let agent1_config = AgentConfig::builder("agent1")
        .system_prompt("You are agent 1")
        .build();
    let agent1 = Agent::new(agent1_config).with_llm_client(mock_llm.clone());

    let context_manager = Arc::new(TokenBudgetManager::new(24_000, 3.0));

    let workflow = Workflow::builder()
        .name("checkpoint_test".to_string())
        .with_chat_history(context_manager.clone())
        .with_max_context_tokens(24_000)
        .with_input_output_ratio(3.0)
        .add_step(Box::new(AgentStep::from_agent(
            agent1,
            "agent1".to_string(),
        )))
        .initial_input(json!("Hello"))
        .build();

    // Get context reference before execution
    let context_ref = workflow.context().cloned().expect("Should have context");

    // Execute first step
    let runtime = Runtime::new();
    let _run1 = runtime.execute(workflow).await;

    // Checkpoint the context using the reference we saved
    let checkpoint = {
        let ctx = context_ref.read().unwrap();
        ctx.clone()
    };

    // Verify checkpoint contains history
    assert!(!checkpoint.chat_history.is_empty());
    assert_eq!(checkpoint.max_context_tokens, 24_000);
    assert_eq!(checkpoint.input_output_ratio, 3.0);

    // Serialize checkpoint (simulating persistence)
    let serialized = serde_json::to_string(&checkpoint).expect("Should serialize");
    println!("Checkpoint size: {} bytes", serialized.len());

    // Deserialize checkpoint (simulating restoration)
    let restored_checkpoint: WorkflowContext =
        serde_json::from_str(&serialized).expect("Should deserialize");

    // Create new workflow with restored context
    let agent2_config = AgentConfig::builder("agent2")
        .system_prompt("You are agent 2")
        .build();
    let agent2 = Agent::new(agent2_config).with_llm_client(mock_llm);

    let restored_workflow = Workflow::builder()
        .name("checkpoint_test_restored".to_string())
        .with_restored_context(restored_checkpoint)
        .add_step(Box::new(AgentStep::from_agent(
            agent2,
            "agent2".to_string(),
        )))
        .initial_input(json!("Continue"))
        .build();

    // Verify restored context
    let ctx_ref = restored_workflow.context().expect("Should have context");
    {
        let ctx = ctx_ref.read().unwrap();
        assert!(!ctx.chat_history.is_empty());
        assert_eq!(ctx.max_context_tokens, 24_000);
    } // Release lock before await

    // Keep reference before execution
    let restored_ctx_ref = restored_workflow
        .context()
        .cloned()
        .expect("Should have context");

    // Execute with restored context
    let _run2 = runtime.execute(restored_workflow).await;

    // Verify new messages were added to history
    let final_history = restored_ctx_ref.read().unwrap();
    assert!(final_history.chat_history.len() > 1);
}

#[tokio::test]
async fn test_external_checkpoint_workflow() {
    // Simulate external checkpoint management (e.g., database, Redis, file)
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Response 1")
            .with_response("Response 2")
            .with_response("Response 3"),
    );

    // Step 1: Run workflow and save checkpoint externally
    let workflow1 = Workflow::builder()
        .name("external_checkpoint".to_string())
        .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("agent1")
                    .system_prompt("Agent 1")
                    .build(),
            )
            .with_llm_client(mock_llm.clone()),
            "agent1".to_string(),
        )))
        .initial_input(json!("Start"))
        .build();

    let runtime = Runtime::new();
    let ctx_ref = workflow1.context().cloned().expect("Should have context");
    let _run1 = runtime.execute(workflow1).await;

    // External checkpoint: Get context reference and serialize
    let external_checkpoint = {
        let ctx = ctx_ref.read().unwrap();
        serde_json::to_string(&*ctx).expect("Should serialize")
    };

    // Save to "database" (simulated)
    let mut checkpoint_db = std::collections::HashMap::new();
    checkpoint_db.insert("external_checkpoint", external_checkpoint.clone());

    // Step 2: Later, load checkpoint and continue
    let loaded_checkpoint: WorkflowContext =
        serde_json::from_str(&checkpoint_db["external_checkpoint"]).expect("Should load");

    let workflow2 = Workflow::builder()
        .name("external_checkpoint_continued".to_string())
        .with_restored_context(loaded_checkpoint)
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("agent2")
                    .system_prompt("Agent 2")
                    .build(),
            )
            .with_llm_client(mock_llm.clone()),
            "agent2".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("agent3")
                    .system_prompt("Agent 3")
                    .build(),
            )
            .with_llm_client(mock_llm),
            "agent3".to_string(),
        )))
        .initial_input(json!("Continue from checkpoint"))
        .build();

    let ctx_ref2 = workflow2.context().cloned().expect("Should have context");
    let _run2 = runtime.execute(workflow2).await;

    // Verify accumulated history
    let ctx = ctx_ref2.read().unwrap();
    assert!(ctx.chat_history.len() >= 3);
}

#[tokio::test]
async fn test_checkpoint_preserves_token_settings() {
    // Test different token configurations are preserved
    let configs = vec![
        (24_000, 3.0, "24k_3to1"),
        (128_000, 4.0, "128k_4to1"),
        (200_000, 1.0, "200k_1to1"),
    ];

    for (tokens, ratio, name) in configs {
        let workflow = Workflow::builder()
            .name(name.to_string())
            .with_chat_history(Arc::new(TokenBudgetManager::new(tokens, ratio)))
            .with_max_context_tokens(tokens)
            .with_input_output_ratio(ratio)
            .build();

        let checkpoint = workflow.checkpoint_context().expect("Should have context");

        assert_eq!(checkpoint.max_context_tokens, tokens);
        assert_eq!(checkpoint.input_output_ratio, ratio);

        // Verify calculated budgets
        let expected_input = (tokens as f64 * ratio / (ratio + 1.0)) as usize;
        let expected_output = (tokens as f64 / (ratio + 1.0)) as usize;

        assert_eq!(checkpoint.max_input_tokens(), expected_input);
        assert_eq!(checkpoint.max_output_tokens(), expected_output);

        // Test serialization round-trip
        let serialized = serde_json::to_string(&checkpoint).unwrap();
        let deserialized: WorkflowContext = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.max_context_tokens, tokens);
        assert_eq!(deserialized.input_output_ratio, ratio);
    }
}

#[test]
fn test_workflow_context_serialization() {
    // Test that WorkflowContext can be serialized/deserialized
    let mut context = WorkflowContext::with_token_budget(24_000, 3.0);
    context.append_messages(vec![
        llm::ChatMessage::system("System prompt"),
        llm::ChatMessage::user("User message"),
        llm::ChatMessage::assistant("Assistant response"),
    ]);

    // Serialize
    let json = serde_json::to_string(&context).expect("Should serialize");
    assert!(json.contains("24000"));
    assert!(json.contains("3"));

    // Deserialize
    let restored: WorkflowContext = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(restored.chat_history.len(), 3);
    assert_eq!(restored.max_context_tokens, 24_000);
    assert_eq!(restored.input_output_ratio, 3.0);
    assert_eq!(restored.chat_history[0].content, "System prompt");
}

#[tokio::test]
async fn test_restore_workflow_without_context_manager() {
    // When restoring from checkpoint, you don't need to provide context_manager again
    let mock_llm = Arc::new(llm::MockLlmClient::new().with_response("Restored response"));

    let mut initial_context = WorkflowContext::with_token_budget(24_000, 3.0);
    initial_context.append_messages(vec![
        llm::ChatMessage::system("Previous system"),
        llm::ChatMessage::user("Previous user"),
    ]);

    // Build workflow with restored context (no context_manager needed)
    let workflow = Workflow::builder()
        .name("restored_without_manager".to_string())
        .with_restored_context(initial_context)
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(AgentConfig::builder("agent").system_prompt("Agent").build())
                .with_llm_client(mock_llm),
            "agent".to_string(),
        )))
        .initial_input(json!("Continue"))
        .build();

    let runtime = Runtime::new();
    let ctx_ref = workflow.context().cloned().expect("Should have context");
    let run = runtime.execute(workflow).await;

    assert_eq!(run.state, WorkflowState::Completed);

    // Verify history preserved
    let ctx = ctx_ref.read().unwrap();
    assert!(ctx.chat_history.len() >= 2);
}
