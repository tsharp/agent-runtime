use crate::tools::registry::Tool;
use crate::types::ToolExecutionResult;
use async_trait::async_trait;
use futures::future::BoxFuture;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

type ToolExecutor = Arc<
    dyn Fn(HashMap<String, JsonValue>) -> BoxFuture<'static, ToolExecutionResult> + Send + Sync,
>;

/// A native (in-memory) tool implemented as a Rust async function
///
/// Native tools execute directly in the runtime process with no IPC overhead.
/// They are defined as async closures that accept parameters and return results.
pub struct NativeTool {
    name: String,
    description: String,
    input_schema: JsonValue,
    executor: ToolExecutor,
}

impl NativeTool {
    /// Create a new native tool
    ///
    /// # Arguments
    /// * `name` - Unique identifier for the tool
    /// * `description` - Human-readable description
    /// * `input_schema` - JSON Schema describing input parameters
    /// * `executor` - Async function that executes the tool
    pub fn new<F, Fut>(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: JsonValue,
        executor: F,
    ) -> Self
    where
        F: Fn(HashMap<String, JsonValue>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolExecutionResult> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            executor: Arc::new(move |params| Box::pin(executor(params))),
        }
    }
}

#[async_trait]
impl Tool for NativeTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> JsonValue {
        self.input_schema.clone()
    }

    async fn execute(&self, params: HashMap<String, JsonValue>) -> ToolExecutionResult {
        (self.executor)(params).await
    }
}

impl std::fmt::Debug for NativeTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .finish()
    }
}
