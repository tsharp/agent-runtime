# MCP (Model Context Protocol) Tool Integration

## Current Status: SCAFFOLD COMPLETE ✅ / IMPLEMENTATION PENDING 🚧

### What's Done

✅ **Complete API Structure**
- `McpClient` - Client for connecting to MCP servers
- `McpTool` - Wrapper implementing our `Tool` trait  
- `McpToolInfo` - Metadata about discovered tools
- Full integration with existing `ToolRegistry` system

✅ **Compiles Successfully**
- All types and methods defined
- Proper error handling signatures
- Async/await patterns in place

✅ **Well Documented**
- Comprehensive doc comments
- Usage examples in code
- Clear TODO comments marking what needs implementation

### What's Needed

🚧 **rust-mcp-sdk Integration**

The `rust-mcp-sdk` crate is added as a dependency, but we need to:

1. **Understand the SDK API Structure**
   - Study the documentation at https://docs.rs/rust-mcp-sdk
   - Find examples or look at source code
   - Identify the main types (Client, Transport, Request/Response)

2. **Implement McpClient::new_stdio()**
   ```rust
   // Pseudo-code of what needs to happen:
   async fn new_stdio(command: &str, args: &[&str]) -> Result<Arc<Self>, String> {
       // Spawn process
       let child = tokio::process::Command::new(command)
           .args(args)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .spawn()?;
       
       // Create transport from stdin/stdout
       let transport = StdioTransport::new(child);
       
       // Create MCP client
       let client = rust_mcp_sdk::Client::new(transport);
       
       // Perform handshake
       client.initialize().await?;
       
       // Return wrapped client
       Ok(Arc::new(Self { inner: client }))
   }
   ```

3. **Implement list_tools()**
   ```rust
   async fn list_tools(&self) -> Result<Vec<MCPToolInfo>, String> {
       // Send tools/list request
       let response = self.inner.request("tools/list", EmptyParams).await?;
       
       // Parse response
       let tools: Vec<Tool> = response.tools;
       
       // Convert to our format
       tools.into_iter().map(|t| MCPToolInfo {
           name: t.name,
           description: t.description,
           input_schema: t.input_schema,
       }).collect()
   }
   ```

4. **Implement call_tool()**
   ```rust
   async fn call_tool(name: &str, args: HashMap<String, JsonValue>) 
       -> Result<JsonValue, String> 
   {
       // Build request
       let request = ToolCallRequest {
           name: name.to_string(),
           arguments: args,
       };
       
       // Send to server
       let response = self.inner.request("tools/call", request).await?;
       
       // Return result
       Ok(response.result)
   }
   ```

### How to Use (Once Implemented)

```rust
use agent_runtime::{McpClient, McpTool, ToolRegistry};
use std::sync::Arc;

// Connect to filesystem MCP server
let mcp_client = McpClient::new_stdio(
    "npx",
    &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
).await?;

// Discover tools
let mcp_tools = mcp_client.list_tools().await?;
println!("Found {} MCP tools", mcp_tools.len());

// Create tool registry with both native and MCP tools
let mut registry = ToolRegistry::new();

// Add native tools
registry.register(NativeTool::new("add", ...));

// Add MCP tools
for info in mcp_tools {
    let tool = McpTool::from_info(info, Arc::clone(&mcp_client));
    registry.register(Arc::new(tool));
}

// Use in agent
let agent = Agent::new(
    AgentConfig::builder("assistant")
        .tools(Arc::new(registry))
        .build()
).with_llm_client(llm);
```

### Next Steps

1. Open https://docs.rs/rust-mcp-sdk and study the API
2. Look for examples in the SDK repository
3. Implement the three TODO methods
4. Create `mcp_tools_demo.rs` to test it
5. Update documentation with working example

### Alternative Approach

If rust-mcp-sdk proves difficult to use, we could:
- Implement the MCP protocol directly (it's JSON-RPC over stdio/HTTP)
- Use a simpler transport library
- Create our own MCP client from scratch

The MCP spec is at: https://modelcontextprotocol.io/docs/specification
