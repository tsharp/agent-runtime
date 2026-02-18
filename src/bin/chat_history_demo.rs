// Example demonstrating workflow-level chat history management
// This shows how agents can automatically share conversation context
use agent_runtime::*;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Workflow Chat History Demo ===\n");

    // Create a mock LLM client (in production, use real LLM)
    let mock_llm = Arc::new(
        llm::MockLlmClient::new()
            .with_response("Hello! I'm the research agent. Based on my analysis, the key factors are A, B, and C.")
            .with_response("As the analysis agent, I've reviewed the research. Factor A seems most critical.")
            .with_response("I'm the summary agent. In conclusion: Focus on Factor A as identified in previous analysis."),
    );

    // Configure agents with shared LLM
    let researcher = Agent::new(
        AgentConfig::builder("researcher")
            .system_prompt("You are a research agent. Analyze the topic.")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let analyzer = Agent::new(
        AgentConfig::builder("analyzer")
            .system_prompt("You are an analysis agent. Review previous findings.")
            .build(),
    )
    .with_llm_client(mock_llm.clone());

    let summarizer = Agent::new(
        AgentConfig::builder("summarizer")
            .system_prompt("You are a summary agent. Wrap up the conversation.")
            .build(),
    )
    .with_llm_client(mock_llm);

    println!("📊 Creating workflow with automatic chat history management");
    println!("   • Context: 24k tokens, 3:1 input/output ratio");
    println!("   • Strategy: TokenBudgetManager");
    println!();

    // Create workflow with token budget manager
    // This handles ANY context size - just configure it!
    let context_manager = Arc::new(TokenBudgetManager::new(24_000, 3.0));

    let workflow = Workflow::builder()
        .name("research_workflow".to_string())
        .with_chat_history(context_manager)
        .with_max_context_tokens(24_000)
        .with_input_output_ratio(3.0)
        .add_step(Box::new(AgentStep::from_agent(
            researcher,
            "researcher".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            analyzer,
            "analyzer".to_string(),
        )))
        .add_step(Box::new(AgentStep::from_agent(
            summarizer,
            "summarizer".to_string(),
        )))
        .initial_input(json!("Analyze the impact of AI on software development"))
        .build();

    // Verify context configuration
    if let Some(context_arc) = &workflow.context {
        let context = context_arc.read().unwrap();
        println!("✅ Workflow context configured:");
        println!("   • Max context: {} tokens", context.max_context_tokens);
        println!("   • Input budget: {} tokens", context.max_input_tokens());
        println!("   • Output budget: {} tokens", context.max_output_tokens());
        println!();
    }

    let context_ref = workflow.context.clone();

    // Execute workflow
    println!("🚀 Executing workflow with 3 agents...\n");
    let runtime = Runtime::new();
    let run = runtime.execute(workflow).await;

    // Print results
    println!("📝 Workflow Results:");
    println!("   • State: {:?}", run.state);
    println!("   • Steps completed: {}", run.steps.len());
    println!();

    for (i, step) in run.steps.iter().enumerate() {
        println!("   Step {}: {}", i + 1, step.step_name);
        if let Some(output) = &step.output {
            if let Some(text) = output.as_str() {
                println!("      Output: {}", text);
            }
        }
        println!();
    }

    // Show accumulated chat history
    if let Some(context_arc) = context_ref {
        let context = context_arc.read().unwrap();
        let history = context.history();

        println!("💬 Accumulated Chat History ({} messages):", history.len());
        println!("   • All agents shared this conversation context");
        println!("   • Each agent saw previous agents' responses");
        println!("   • History automatically managed within token budget");
        println!();

        for (i, msg) in history.iter().enumerate() {
            println!("   [{:?}] {}", msg.role, 
                if msg.content.len() > 80 {
                    format!("{}...", &msg.content[..80])
                } else {
                    msg.content.clone()
                }
            );
            if i >= 5 {
                println!("   ... ({} more messages)", history.len() - i - 1);
                break;
            }
        }
        println!();
    }

    println!("✨ Demo complete!");
    println!();
    println!("💡 Key Features:");
    println!("   • Configurable context size: 24k, 128k, 200k, or any size!");
    println!("   • Flexible input/output ratios: 1:1, 3:1, 4:1, 9:1, or custom");
    println!("   • Automatic pruning when approaching limits");
    println!("   • Multiple strategies: TokenBudget, SlidingWindow, or custom");
    println!("   • Backward compatible: opt-in only, existing code works");

    Ok(())
}
