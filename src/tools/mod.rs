//! Tool system: registry, native tools, MCP integration, and loop detection.

pub mod builtin;
pub mod loop_detection;
pub mod mcp;
pub mod native;
pub mod registry;

pub use builtin::{CalculatorTool, EchoTool};
pub use loop_detection::{ToolCallTracker, ToolLoopDetectionConfig};
pub use mcp::{McpClient, McpTool, McpToolInfo};
pub use native::NativeTool;
pub use registry::{Tool, ToolRegistry};
