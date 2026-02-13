use agent_runtime::llm::{OpenAIClient, ChatClient, ChatMessage, ChatRequest};

#[tokio::main]
async fn main() {
    println!("=== LLM Client Demo ===\n");
    
    // Get API key from environment
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable not set");
    
    // Create client
    let client = OpenAIClient::with_model(api_key, "gpt-3.5-turbo");
    
    println!("Provider: {}", client.provider());
    println!("Model: {}\n", client.model());
    
    // Build a simple request
    let request = ChatRequest::new(vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Say hello in exactly 5 words."),
    ]);
    
    println!("Sending request...");
    
    // Send request
    match client.chat(request).await {
        Ok(response) => {
            println!("\n✅ Success!");
            println!("Response: {}", response.content);
            println!("Model: {}", response.model);
            
            if let Some(usage) = response.usage {
                println!("\nUsage:");
                println!("  Prompt tokens: {}", usage.prompt_tokens);
                println!("  Completion tokens: {}", usage.completion_tokens);
                println!("  Total tokens: {}", usage.total_tokens);
            }
            
            if let Some(finish_reason) = response.finish_reason {
                println!("Finish reason: {}", finish_reason);
            }
        }
        Err(e) => {
            eprintln!("\n❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}
