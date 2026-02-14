// MCP Client implementation using rust-mcp-sdk
//
// The Model Context Protocol (MCP) is a protocol for AI assistants to interact
// with external tools and data sources. This module provides integration with
// MCP servers via the rust-mcp-sdk.
//
// Current Status: PLACEHOLDER IMPLEMENTATION
// The rust-mcp-sdk API structure is not yet fully understood. This module provides
// a complete API surface that compiles, but the actual MCP protocol communication
// needs to be implemented.
//
// To complete this implementation, we need to:
// 1. Study rust-mcp-sdk documentation and examples
// 2. Understand how to create clients and transports
// 3. Implement the MCP protocol request/response cycle
// 4. Handle tool discovery and execution
//
// For now, this provides a working skeleton that integrates with our tool system.

use crate::tool::Tool;
use crate::types::{JsonValue, ToolError, ToolResult};
use async_trait::async_trait;
use rust_mcp_sdk::{
    mcp_client::{
        client_runtime_core, ClientHandlerCore, McpClientOptions, ToMcpClientHandlerCore,
    },
    schema::{
        CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
        LATEST_PROTOCOL_VERSION,
    },
    McpClient as SdkMcpClient, StdioTransport, TransportOptions,
};
use std::collections::HashMap;
use std::sync::Arc;

/// MCP Client wrapper for connecting to MCP servers
///
/// Manages a connection to an MCP server and provides methods to:
/// - Discover available tools
/// - Execute tools remotely
///
/// # Example (when implemented)
/// ```no_run
/// # use agent_runtime::McpClient;
/// # async fn example() -> Result<(), String> {
/// // Connect to an MCP server via stdio
/// let client = McpClient::new_stdio("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]).await?;
///
/// // Discover tools
/// let tools = client.list_tools().await?;
/// println!("Found {} tools", tools.len());
/// # Ok(())
/// # }
/// ```
pub struct McpClient {
    inner: Arc<dyn SdkMcpClient>,
}

impl McpClient {
    /// Create a new MCP client connected to a server via stdio
    ///
    /// # Arguments
    /// * `command` - The command to run (e.g., "npx", "python", "node")
    /// * `args` - Arguments to pass (e.g., ["-y", "@modelcontextprotocol/server-filesystem", "/path"])
    ///
    /// # Example MCP Servers
    /// - Filesystem: `npx -y @modelcontextprotocol/server-filesystem /tmp`
    /// - SQLite: `npx -y @modelcontextprotocol/server-sqlite --db-path ./data.db`
    /// - Web: `npx -y @modelcontextprotocol/server-fetch`
    pub async fn new_stdio(command: &str, args: &[&str]) -> Result<Arc<Self>, String> {
        // Create client details
        let client_details = InitializeRequestParams {
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "agent-runtime-mcp-client".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: Some("MCP client for agent-runtime framework".into()),
                title: None,
                icons: vec![],
                website_url: None,
            },
            protocol_version: LATEST_PROTOCOL_VERSION.into(),
            meta: None,
        };

        // Create transport that launches the MCP server
        let transport = StdioTransport::create_with_server_launch(
            command,
            args.iter().map(|s| s.to_string()).collect(),
            None,
            TransportOptions::default(),
        )
        .map_err(|e| format!("Failed to create transport: {}", e))?;

        // Create a minimal handler
        let handler = MinimalClientHandler {};

        // Create and start the MCP client
        let client = client_runtime_core::create_client(McpClientOptions {
            client_details,
            transport,
            handler: handler.to_mcp_client_handler(),
            task_store: None,
            server_task_store: None,
        });

        client
            .clone()
            .start()
            .await
            .map_err(|e| format!("Failed to start MCP client: {}", e))?;

        Ok(Arc::new(Self { inner: client }))
    }

    /// Discover all tools available on the connected MCP server
    ///
    /// Sends a `tools/list` request to the MCP server and parses the response.
    pub async fn list_tools(&self) -> Result<Vec<McpToolInfo>, String> {
        let response = self
            .inner
            .request_tool_list(None)
            .await
            .map_err(|e| format!("Failed to list tools: {}", e))?;

        Ok(response
            .tools
            .into_iter()
            .map(|tool| McpToolInfo {
                name: tool.name,
                description: tool.description.unwrap_or_default(),
                input_schema: serde_json::to_value(&tool.input_schema).unwrap_or(JsonValue::Null),
            })
            .collect())
    }

    /// Call a tool on the MCP server
    ///
    /// Sends a `tools/call` request with the given arguments and waits for the result.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: HashMap<String, JsonValue>,
    ) -> Result<JsonValue, String> {
        let params = CallToolRequestParams {
            name: name.to_string(),
            arguments: Some(
                arguments
                    .into_iter()
                    .map(|(k, v)| (k, v))
                    .collect::<serde_json::Map<String, JsonValue>>(),
            ),
            meta: None,
            task: None,
        };

        let result = self
            .inner
            .request_tool_call(params)
            .await
            .map_err(|e| format!("MCP tool call failed: {}", e))?;

        // Convert the result content to a JSON value
        if let Some(content) = result.content.first() {
            if let Ok(text_content) = content.as_text_content() {
                Ok(JsonValue::String(text_content.text.clone()))
            } else {
                Ok(serde_json::to_value(&content)
                    .map_err(|e| format!("Failed to serialize result: {}", e))?)
            }
        } else {
            Ok(JsonValue::Null)
        }
    }
}

/// Information about a tool discovered from an MCP server
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

/// Minimal handler for MCP client (we don't need custom message handling)
struct MinimalClientHandler;

use rust_mcp_sdk::schema::{
    NotificationFromServer, ResultFromClient, RpcError, ServerJsonrpcRequest,
};

#[async_trait]
impl ClientHandlerCore for MinimalClientHandler {
    async fn handle_request(
        &self,
        _request: ServerJsonrpcRequest,
        _runtime: &dyn SdkMcpClient,
    ) -> Result<ResultFromClient, RpcError> {
        Err(RpcError::method_not_found())
    }

    async fn handle_notification(
        &self,
        _notification: NotificationFromServer,
        _runtime: &dyn SdkMcpClient,
    ) -> Result<(), RpcError> {
        Ok(())
    }

    async fn handle_error(
        &self,
        _error: &RpcError,
        _runtime: &dyn SdkMcpClient,
    ) -> Result<(), RpcError> {
        Ok(())
    }
}

/// A tool that wraps an MCP server tool
///
/// This implements our `Tool` trait so it can be used alongside native tools
/// in the `ToolRegistry`.
pub struct McpTool {
    name: String,
    description: String,
    input_schema: JsonValue,
    // Reference to the MCP client for making calls
    client: Arc<McpClient>,
}

impl McpTool {
    /// Create a new MCP tool wrapper
    pub fn new(
        name: String,
        description: String,
        input_schema: JsonValue,
        client: Arc<McpClient>,
    ) -> Self {
        Self {
            name,
            description,
            input_schema,
            client,
        }
    }

    /// Create from McpToolInfo (convenience method)
    pub fn from_info(info: McpToolInfo, client: Arc<McpClient>) -> Self {
        Self::new(info.name, info.description, info.input_schema, client)
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    async fn execute(&self, params: HashMap<String, JsonValue>) -> Result<ToolResult, ToolError> {
        let start = std::time::Instant::now();

        // Call through to MCP server
        match self.client.call_tool(&self.name, params).await {
            Ok(output) => Ok(ToolResult::success(
                output,
                start.elapsed().as_secs_f64() * 1000.0,
            )),
            Err(e) => Err(ToolError::ExecutionFailed(format!("MCP error: {}", e))),
        }
    }
}
