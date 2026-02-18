//! Comprehensive demo showing workflow chat history, checkpointing, and sub-workflows
use agent_runtime::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Workflow Features Demo ===\n");

    // ========================================
    // Part 1: Multi-Stage Workflow with Shared Context
    // ========================================
    println!("📊 Part 1: Multi-Stage Research Workflow");
    println!("   • Three agents collaborate on research");
    println!("   • All share conversation history");
    println!("   • Token budget: 24k (18k input / 6k output)\n");

    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Research Agent: I've analyzed the topic. Key points are A, B, C.")
            .with_response(
                "Analysis Agent: Based on the research mentioning A, B, C, factor A is critical.",
            )
            .with_response(
                "Summary Agent: Synthesizing conversation history: Focus on factor A.",
            )
            .with_response("Main Agent: Now executing detailed analysis sub-workflow...")
            .with_response("Detail Agent 1: Deep dive on factor A shows X, Y, Z.")
            .with_response("Detail Agent 2: Cross-referencing findings: Z is most important.")
            .with_response(
                "Main Agent: Based on sub-workflow findings (Z is key), final recommendation is...",
            ),
    );

    // Stage 1: Initial research workflow
    let workflow1 = Workflow::builder()
        .name("research_workflow".to_string())
        .with_chat_history(Arc::new(TokenBudgetManager::new(24_000, 3.0)))
        .with_max_context_tokens(24_000)
        .with_input_output_ratio(3.0)
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("researcher")
                    .system_prompt("You are a research agent")
                    .build(),
            )
            .with_llm_client(mock_llm.clone()),
            "researcher".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("analyzer")
                    .system_prompt("You are an analysis agent")
                    .build(),
            )
            .with_llm_client(mock_llm.clone()),
            "analyzer".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("summarizer")
                    .system_prompt("You are a summary agent")
                    .build(),
            )
            .with_llm_client(mock_llm.clone()),
            "summarizer".to_string(),
        )))
        .initial_input(json!("Analyze the impact of AI on software development"))
        .build();

    let ctx_ref1 = workflow1.context().cloned().expect("Should have context");

    let runtime = Runtime::new();
    let run1 = runtime.execute(workflow1).await;

    println!("✅ Stage 1 Complete: {} steps", run1.steps.len());

    // ========================================
    // Part 2: Checkpoint and Resume
    // ========================================
    println!("\n💾 Part 2: Checkpointing Conversation State");

    // Checkpoint the context
    let checkpoint = {
        let ctx = ctx_ref1.read().unwrap();
        ctx.clone()
    };

    println!("   • Captured {} messages in checkpoint", checkpoint.chat_history.len());
    println!("   • Context size: {} tokens", checkpoint.max_context_tokens);

    // Simulate saving to external storage
    let serialized = serde_json::to_string(&checkpoint)?;
    println!("   • Serialized: {} bytes", serialized.len());

    // Simulate loading from storage later
    let loaded_checkpoint: WorkflowContext = serde_json::from_str(&serialized)?;
    println!("   • Deserialized successfully\n");

    // ========================================
    // Part 3: Resume with Sub-Workflow
    // ========================================
    println!("🔄 Part 3: Resuming with Sub-Workflow");
    println!("   • Restore conversation state");
    println!("   • Execute sub-workflow for detailed analysis");
    println!("   • Sub-workflow shares parent context\n");

    // Create sub-workflow builder
    let mock_sub = mock_llm.clone();
    let detail_workflow_builder = move || {
        let detail1 = Agent::new(
            AgentConfig::builder("detail1")
                .system_prompt("You provide detailed analysis 1")
                .build(),
        )
        .with_llm_client(mock_sub.clone());

        let detail2 = Agent::new(
            AgentConfig::builder("detail2")
                .system_prompt("You provide detailed analysis 2")
                .build(),
        )
        .with_llm_client(mock_sub.clone());

        Workflow::builder()
            .name("detail_analysis".to_string())
            .add_step(Box::new(AgentStep::from_agent(
                detail1,
                "detail1".to_string(),
            )))
            .add_step(Box::new(AgentStep::from_agent(
                detail2,
                "detail2".to_string(),
            )))
            .build()
    };

    // Resume workflow with restored context + sub-workflow
    let workflow2 = Workflow::builder()
        .name("resumed_with_subworkflow".to_string())
        .with_restored_context(loaded_checkpoint)
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("main_agent")
                    .system_prompt("You are the main coordinating agent")
                    .build(),
            )
            .with_llm_client(mock_llm.clone()),
            "main_agent".to_string(),
        )))
        .add_step(Box::new(SubWorkflowStep::new(
            "detail_analysis".to_string(),
            detail_workflow_builder,
        )))
        .add_step(Box::new(AgentStep::from_agent(
            Agent::new(
                AgentConfig::builder("final_agent")
                    .system_prompt("You are the final synthesis agent")
                    .build(),
            )
            .with_llm_client(mock_llm),
            "final_agent".to_string(),
        )))
        .initial_input(json!("Continue from checkpoint"))
        .build();

    let ctx_ref2 = workflow2.context().cloned().expect("Should have context");

    let run2 = runtime.execute(workflow2).await;

    println!("✅ Stage 2 Complete: {} steps", run2.steps.len());
    println!("   • Included sub-workflow with 2 internal steps");

    // ========================================
    // Part 4: Inspect Final State
    // ========================================
    println!("\n📋 Part 4: Final Conversation History");

    let final_ctx = ctx_ref2.read().unwrap();
    let final_history = final_ctx.history();

    println!("   • Total messages: {}", final_history.len());
    println!("   • Messages from:");
    println!("      - Original workflow (3 agents)");
    println!("      - Checkpoint restoration");
    println!("      - Main agent coordination");
    println!("      - Sub-workflow agents (2 agents)");
    println!("      - Final synthesis");
    println!("\n   • All agents shared the same conversation context!");

    // Show sample of history
    println!("\n   Sample messages:");
    for (_i, msg) in final_history.iter().take(6).enumerate() {
        let truncated = if msg.content.len() > 60 {
            format!("{}...", &msg.content[..60])
        } else {
            msg.content.clone()
        };
        println!("   [{:?}] {}", msg.role, truncated);
    }

    if final_history.len() > 6 {
        println!("   ... ({} more messages)", final_history.len() - 6);
    }

    // ========================================
    // Summary
    // ========================================
    println!("\n✨ Demo Summary:");
    println!("   ✅ Multi-agent collaboration with shared context");
    println!("   ✅ External checkpointing (serialize/deserialize)");
    println!("   ✅ Workflow resumption from checkpoint");
    println!("   ✅ Sub-workflows with context sharing");
    println!("   ✅ Flexible token management (24k, 128k, 200k, or any size)");
    println!("   ✅ Configurable ratios (3:1, 4:1, 1:1, or custom)");
    println!("\n💡 Key Features:");
    println!("   • WorkflowContext can be checkpointed externally");
    println!("   • Sub-workflows automatically share parent context");
    println!("   • Full e2e workflow maintains conversation history");
    println!("   • Supports any token size and ratio configuration");

    Ok(())
}
