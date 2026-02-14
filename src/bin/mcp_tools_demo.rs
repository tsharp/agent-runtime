use agent_runtime::{McpClient, McpTool, Tool};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MCP Tools Demo ===\n");

    // Test 1: Connect to the "everything" MCP server
    println!("🔌 Connecting to @modelcontextprotocol/server-everything...");
    let client =
        McpClient::new_stdio("npx", &["-y", "@modelcontextprotocol/server-everything"]).await?;
    println!("✅ Connected!\n");

    // Test 2: Discover tools
    println!("🔍 Discovering available tools...");
    let tools_info = client.list_tools().await?;
    println!("✅ Found {} tools:\n", tools_info.len());

    for (idx, tool_info) in tools_info.iter().enumerate() {
        println!(
            "  {}. {} - {}",
            idx + 1,
            tool_info.name,
            tool_info.description
        );
    }
    println!();

    // Test 3: Create McpTool wrappers
    println!("🔧 Creating tool wrappers...");
    let tools: Vec<_> = tools_info
        .iter()
        .map(|info| Arc::new(McpTool::from_info(info.clone(), Arc::clone(&client))))
        .collect();
    println!("✅ Created {} tool wrappers\n", tools.len());

    // Test 4: Test the get-sum tool
    println!("🧮 Testing 'get-sum' tool: 42 + 13");
    if let Some(sum_tool) = tools.iter().find(|t| t.name() == "get-sum") {
        let mut params = std::collections::HashMap::new();
        params.insert("a".to_string(), serde_json::json!(42));
        params.insert("b".to_string(), serde_json::json!(13));

        let result = sum_tool.execute(params).await;
        match result {
            Ok(tool_result) => {
                println!("✅ Result: {}", tool_result.output);
                println!("   Duration: {:.2} ms", tool_result.duration_ms);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    } else {
        println!("⚠️  'get-sum' tool not found");
    }
    println!();

    // Test 5: Test the echo tool
    println!("📣 Testing 'echo' tool");
    if let Some(echo_tool) = tools.iter().find(|t| t.name() == "echo") {
        let mut params = std::collections::HashMap::new();
        params.insert("message".to_string(), serde_json::json!("Hello from MCP!"));

        let result = echo_tool.execute(params).await;
        match result {
            Ok(tool_result) => {
                println!("✅ Result: {}", tool_result.output);
                println!("   Duration: {:.2} ms", tool_result.duration_ms);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    } else {
        println!("⚠️  'echo' tool not found");
    }
    println!();

    println!("✅ MCP Tools Demo Complete!");

    Ok(())
}
