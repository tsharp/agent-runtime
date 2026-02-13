use agent_runtime::{
    AgentConfig, Agent, Workflow, AgentStep, Runtime,
    llm::{LlamaClient, ChatClient},
};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("=== Workflow Demo ===\n");
    
    // Create LLM client (insecure HTTPS for local dev)
    let llm_client: Arc<dyn ChatClient> = Arc::new(
        LlamaClient::insecure("https://192.168.91.57", "default")
    );
    
    println!("✓ LLM client configured (https://192.168.91.57 - insecure)\n");
    
    // Create agents
    let greeter = Agent::new(
        AgentConfig::builder("greeter")
            .system_prompt("You are a friendly greeter. Say hello and introduce yourself warmly.")
            .build()
    ).with_llm_client(llm_client.clone());
    
    let analyzer = Agent::new(
        AgentConfig::builder("analyzer")
            .system_prompt("You are a thoughtful analyzer. Analyze the input and provide insights.")
            .build()
    ).with_llm_client(llm_client.clone());
    
    let summarizer = Agent::new(
        AgentConfig::builder("summarizer")
            .system_prompt("You are a concise summarizer. Summarize the conversation in 2-3 sentences.")
            .build()
    ).with_llm_client(llm_client.clone());
    
    println!("✓ Created 3 agents: greeter → analyzer → summarizer\n");
    
    // Build workflow
    let workflow = Workflow::builder()
        .step(Box::new(AgentStep::from_agent(greeter, "greeter".to_string())))
        .step(Box::new(AgentStep::from_agent(analyzer, "analyzer".to_string())))
        .step(Box::new(AgentStep::from_agent(summarizer, "summarizer".to_string())))
        .initial_input(serde_json::json!("Hello! I'm interested in learning about AI agents."))
        .build();
    
    println!("✓ Workflow built with 3 sequential steps\n");
    
    // Show mermaid diagram
    println!("Workflow Structure:");
    println!("{}\n", workflow.to_mermaid());
    
    // Create runtime and execute
    let runtime = Runtime::new();
    
    // Execute workflow
    println!("▶ Starting workflow execution...\n");
    println!("{}", "=".repeat(60));
    
    let result = runtime.execute(workflow).await;
    
    println!("\n{}", "=".repeat(60));
    println!("\n✅ Workflow execution complete!\n");
    
    if let Some(output) = &result.final_output {
        println!("Final Output:");
        println!("{}\n", serde_json::to_string_pretty(&output).unwrap());
    }
    
    println!("Steps executed: {}", result.steps.len());
    for (i, step) in result.steps.iter().enumerate() {
        println!("  Step {}: {} ({})", i+1, step.step_name, step.step_type);
        if let Some(time) = step.execution_time_ms {
            println!("    Execution time: {}ms", time);
        }
    }
}
