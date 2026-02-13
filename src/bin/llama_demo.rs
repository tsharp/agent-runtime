use agent_runtime::llm::{LlamaClient, ChatClient, ChatMessage, ChatRequest};

#[tokio::main]
async fn main() {
    println!("=== Llama.cpp Client Demo ===\n");
    
    // Create client pointing to localhost:1234 (e.g., LM Studio, llama.cpp)
    let client = LlamaClient::new("http://localhost:1234", "qwen/qwen3-30b-a3b-2507");
    
    println!("Provider: {}", client.provider());
    println!("Model: {}", client.model());
    println!("Connecting to localhost:1234...\n");
    
    // Build a simple request
    let request = ChatRequest::new(vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Say hello in exactly 5 words."),
    ])
    .with_temperature(0.7);
    
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
            eprintln!("\nMake sure llama.cpp server is running on port 1234:");
            eprintln!("  e.g., LM Studio or: ./server -m models/llama-2-7b-chat.gguf --port 1234");
            std::process::exit(1);
        }
    }
}
