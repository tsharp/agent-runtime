use agent_runtime::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Advanced Context Strategy Demonstrations ===\n");

    // Demo 1: MessageTypeManager
    demo_message_type_manager().await;

    println!("\n{}\n", "=".repeat(80));

    // Demo 2: SummarizationManager
    demo_summarization_manager().await;

    println!("\n{}\n", "=".repeat(80));

    // Demo 3: Strategy Comparison
    demo_strategy_comparison().await;
}

async fn demo_message_type_manager() {
    println!("DEMO 1: MessageTypeManager - Priority-Based Pruning");
    println!("{}", "-".repeat(80));
    println!("Strategy: Keep system messages + recent user/assistant pairs");
    println!("          Prune old tool calls first\n");

    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Analyzing the data...")
            .with_response("Based on analysis, recommendation: increase budget")
            .with_response("Creating detailed report...")
            .with_response("Report complete with findings"),
    );

    // Create workflow with MessageTypeManager
    // Keep max 10 messages, preserve last 3 user/assistant pairs
    let manager = Arc::new(MessageTypeManager::new(10, 3));

    let agent1 = Agent::new(
        AgentConfig::builder("data_analyzer")
            .system_prompt("You are a data analyzer.")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let agent2 = Agent::new(
        AgentConfig::builder("recommender")
            .system_prompt("You provide recommendations.")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let agent3 = Agent::new(
        AgentConfig::builder("reporter")
            .system_prompt("You create reports.")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let agent4 = Agent::new(
        AgentConfig::builder("finalizer")
            .system_prompt("You finalize outputs.")
            .build(),
    )
    .with_llm_client(mock_llm);

    let workflow = Workflow::builder()
        .name("message_type_demo".to_string())
        .with_chat_history(manager)
        .with_max_context_tokens(1000) // Small context to force pruning
        .with_input_output_ratio(3.0)
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
        .add_step(Box::new(AgentStep::from_agent(
            agent4,
            "agent4".to_string(),
        )))
        .initial_input(json!("Analyze Q4 sales data"))
        .build();

    let ctx_ref = workflow.context().cloned().expect("Has context");

    let runtime = Runtime::new();
    let _result = runtime.execute(workflow).await;

    let final_ctx = ctx_ref.read().unwrap();
    println!(
        "Final history length: {} messages",
        final_ctx.chat_history.len()
    );
    println!("Messages (last 5):");
    for (i, msg) in final_ctx
        .chat_history
        .iter()
        .rev()
        .take(5)
        .rev()
        .enumerate()
    {
        println!(
            "  {}. [{:?}] {}",
            i + 1,
            msg.role,
            msg.content.chars().take(60).collect::<String>()
        );
    }

    println!("\n✓ MessageTypeManager preserved critical conversation pairs");
    println!("  while pruning less important messages");
}

async fn demo_summarization_manager() {
    println!("DEMO 2: SummarizationManager - Intelligent Compression");
    println!("{}", "-".repeat(80));
    println!("Strategy: When history exceeds threshold, summarize old messages");
    println!("          Keep recent messages untouched\n");

    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Step 1: Research shows market trends favor product A")
            .with_response("Step 2: Competitive analysis reveals gaps in market")
            .with_response("Step 3: Financial model projects 25% growth")
            .with_response("Step 4: Risk assessment identifies supply chain concerns")
            .with_response("Step 5: Recommendation - proceed with phased rollout"),
    );

    // Create workflow with SummarizationManager
    // Threshold: 500 tokens, target summary: 100 tokens, keep last 2 messages
    let manager = Arc::new(SummarizationManager::new(750, 500, 100, 2));

    let agents: Vec<_> = (1..=5)
        .map(|i| {
            Agent::new(
                AgentConfig::builder(format!("step_{}", i))
                    .system_prompt(format!("You are step {} of the analysis pipeline", i))
                    .build(),
            )
            .with_llm_client(mock_llm.clone())
        })
        .collect();

    let mut workflow_builder = Workflow::builder()
        .name("summarization_demo".to_string())
        .with_chat_history(manager)
        .with_max_context_tokens(1000)
        .with_input_output_ratio(3.0);

    for (i, agent) in agents.into_iter().enumerate() {
        workflow_builder = workflow_builder.add_step(Box::new(AgentStep::from_agent(
            agent,
            format!("step_{}", i + 1),
        )));
    }

    let workflow = workflow_builder
        .initial_input(json!("Conduct comprehensive product analysis"))
        .build();

    let ctx_ref = workflow.context().cloned().expect("Has context");

    let runtime = Runtime::new();
    let _result = runtime.execute(workflow).await;

    let final_ctx = ctx_ref.read().unwrap();
    println!(
        "Final history length: {} messages",
        final_ctx.chat_history.len()
    );

    // Check if summarization occurred
    let has_summary = final_ctx
        .chat_history
        .iter()
        .any(|msg| msg.content.contains("Summary of previous conversation"));

    if has_summary {
        println!("\n✓ SummarizationManager created compressed summary:");
        for msg in &final_ctx.chat_history {
            if msg.content.contains("Summary") {
                println!("  {}", msg.content.lines().next().unwrap());
            }
        }
    }

    println!("\nRecent messages (preserved):");
    for (i, msg) in final_ctx
        .chat_history
        .iter()
        .rev()
        .take(2)
        .rev()
        .enumerate()
    {
        if msg.role == Role::Assistant {
            println!(
                "  {}. {}",
                i + 1,
                msg.content.chars().take(70).collect::<String>()
            );
        }
    }
}

async fn demo_strategy_comparison() {
    println!("DEMO 3: Strategy Comparison");
    println!("{}", "-".repeat(80));
    println!("Comparing different strategies on the same workflow\n");

    // Create a mock LLM with enough responses for multiple workflows
    let create_llm = || {
        Arc::new(
            llm::MockLlmClient::new()
                .with_response("Response 1")
                .with_response("Response 2")
                .with_response("Response 3")
                .with_response("Response 4")
                .with_response("Response 5"),
        )
    };

    // Strategy 1: TokenBudgetManager
    println!("1. TokenBudgetManager (flexible, ratio-based)");
    let llm1 = create_llm();
    let manager1 = Arc::new(TokenBudgetManager::new(1000, 3.0));
    let result1 = run_workflow_with_manager(manager1, llm1, "token_budget").await;
    println!("   Final messages: {}", result1);

    // Strategy 2: SlidingWindowManager
    println!("2. SlidingWindowManager (simple FIFO)");
    let llm2 = create_llm();
    let manager2 = Arc::new(SlidingWindowManager::new(8));
    let result2 = run_workflow_with_manager(manager2, llm2, "sliding_window").await;
    println!("   Final messages: {}", result2);

    // Strategy 3: MessageTypeManager
    println!("3. MessageTypeManager (priority-based)");
    let llm3 = create_llm();
    let manager3 = Arc::new(MessageTypeManager::new(10, 3));
    let result3 = run_workflow_with_manager(manager3, llm3, "message_type").await;
    println!("   Final messages: {}", result3);

    println!("\n✓ Each strategy has different pruning behavior:");
    println!("  - TokenBudget: Prunes based on estimated token count");
    println!("  - SlidingWindow: Keeps last N messages");
    println!("  - MessageType: Prioritizes conversation pairs");
}

async fn run_workflow_with_manager(
    manager: Arc<dyn ContextManager>,
    llm: Arc<llm::MockLlmClient>,
    name: &str,
) -> usize {
    let agents: Vec<_> = (1..=5)
        .map(|i| {
            Agent::new(
                AgentConfig::builder(format!("agent_{}", i))
                    .system_prompt(format!("Agent {}", i))
                    .build(),
            )
            .with_llm_client(llm.clone())
        })
        .collect();

    let mut workflow_builder = Workflow::builder()
        .name(name.to_string())
        .with_chat_history(manager)
        .with_max_context_tokens(1000)
        .with_input_output_ratio(3.0);

    for (i, agent) in agents.into_iter().enumerate() {
        workflow_builder = workflow_builder.add_step(Box::new(AgentStep::from_agent(
            agent,
            format!("agent_{}", i + 1),
        )));
    }

    let workflow = workflow_builder.initial_input(json!("Test input")).build();

    let ctx_ref = workflow.context().cloned().expect("Has context");

    let runtime = Runtime::new();
    let _result = runtime.execute(workflow).await;

    let final_ctx = ctx_ref.read().unwrap();
    final_ctx.chat_history.len()
}
