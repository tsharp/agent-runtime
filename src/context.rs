use crate::llm::types::ChatMessage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Central workflow context that manages conversation history across steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Conversation history shared across workflow steps
    pub chat_history: Vec<ChatMessage>,

    /// Workflow-level metadata (for resumption)
    pub metadata: WorkflowMetadata,

    /// Maximum context size in tokens (total input + output capacity)
    pub max_context_tokens: usize,

    /// Input to output token ratio (e.g., 3.0 means 3:1 ratio)
    pub input_output_ratio: f64,
}

impl WorkflowContext {
    /// Create a new workflow context with default settings
    pub fn new() -> Self {
        Self {
            chat_history: Vec::new(),
            metadata: WorkflowMetadata::default(),
            max_context_tokens: 128_000, // Default to 128k
            input_output_ratio: 4.0,      // Default 4:1 ratio
        }
    }

    /// Create a workflow context with specific token limits
    pub fn with_token_budget(max_tokens: usize, input_output_ratio: f64) -> Self {
        Self {
            chat_history: Vec::new(),
            metadata: WorkflowMetadata::default(),
            max_context_tokens: max_tokens,
            input_output_ratio,
        }
    }

    /// Calculate the maximum tokens available for input
    pub fn max_input_tokens(&self) -> usize {
        let total = self.max_context_tokens as f64;
        let ratio = self.input_output_ratio;
        // input = total * (ratio / (ratio + 1))
        (total * ratio / (ratio + 1.0)) as usize
    }

    /// Calculate the maximum tokens reserved for output
    pub fn max_output_tokens(&self) -> usize {
        let total = self.max_context_tokens as f64;
        let ratio = self.input_output_ratio;
        // output = total / (ratio + 1)
        (total / (ratio + 1.0)) as usize
    }

    /// Add messages to the chat history
    pub fn append_messages(&mut self, messages: Vec<ChatMessage>) {
        self.chat_history.extend(messages);
        self.metadata.last_updated = Utc::now();
    }

    /// Replace the entire chat history
    pub fn set_history(&mut self, history: Vec<ChatMessage>) {
        self.chat_history = history;
        self.metadata.last_updated = Utc::now();
    }

    /// Get the current chat history
    pub fn history(&self) -> &[ChatMessage] {
        &self.chat_history
    }

    /// Create a fork of this context for sub-workflows (isolated copy)
    pub fn fork(&self) -> Self {
        Self {
            chat_history: self.chat_history.clone(),
            metadata: WorkflowMetadata {
                workflow_id: format!("{}-fork", self.metadata.workflow_id),
                created_at: Utc::now(),
                last_updated: Utc::now(),
                step_count: 0,
            },
            max_context_tokens: self.max_context_tokens,
            input_output_ratio: self.input_output_ratio,
        }
    }
}

impl Default for WorkflowContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about the workflow context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub workflow_id: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub step_count: usize,
}

impl Default for WorkflowMetadata {
    fn default() -> Self {
        Self {
            workflow_id: format!("wf_{}", uuid::Uuid::new_v4()),
            created_at: Utc::now(),
            last_updated: Utc::now(),
            step_count: 0,
        }
    }
}

/// Strategy interface for managing conversation context size
#[async_trait]
pub trait ContextManager: Send + Sync {
    /// Check if context needs pruning based on current state
    async fn should_prune(&self, history: &[ChatMessage], current_tokens: usize) -> bool;

    /// Apply pruning strategy and return new history
    /// Returns the pruned history and the number of tokens freed
    async fn prune(
        &self,
        history: Vec<ChatMessage>,
    ) -> Result<(Vec<ChatMessage>, usize), ContextError>;

    /// Estimate token count for messages
    /// This is an approximation - actual tokenization varies by model
    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize;

    /// Get the name of this strategy
    fn name(&self) -> &str;
}

/// Errors that can occur during context management
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Token estimation failed: {0}")]
    EstimationError(String),

    #[error("Pruning failed: {0}")]
    PruningError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Context too large: {current} tokens, max {max}")]
    ContextTooLarge { current: usize, max: usize },
}

/// No-op context manager that doesn't prune anything
/// Useful for large context models or when external management is preferred
pub struct NoOpManager;

impl NoOpManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextManager for NoOpManager {
    async fn should_prune(&self, _history: &[ChatMessage], _current_tokens: usize) -> bool {
        false // Never prune
    }

    async fn prune(
        &self,
        history: Vec<ChatMessage>,
    ) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        Ok((history, 0)) // Return unchanged
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        // Simple approximation: ~4 characters per token
        messages
            .iter()
            .map(|msg| msg.content.len() / 4)
            .sum::<usize>()
    }

    fn name(&self) -> &str {
        "NoOp"
    }
}

/// Merge strategy for combining sub-workflow context back into parent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Only add the sub-workflow's final output as a message
    AppendResults,

    /// Include all sub-workflow messages (verbose)
    FullMerge,

    /// Compress sub-workflow conversation into a summary (requires implementation)
    SummarizedMerge,

    /// Don't propagate any context back to parent
    Discard,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::AppendResults
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_context_creation() {
        let ctx = WorkflowContext::new();
        assert_eq!(ctx.max_context_tokens, 128_000);
        assert_eq!(ctx.input_output_ratio, 4.0);
        assert!(ctx.chat_history.is_empty());
    }

    #[test]
    fn test_token_budget_calculation_3_to_1() {
        let ctx = WorkflowContext::with_token_budget(24_000, 3.0);
        assert_eq!(ctx.max_input_tokens(), 18_000); // 24k * (3/4) = 18k
        assert_eq!(ctx.max_output_tokens(), 6_000); // 24k * (1/4) = 6k
    }

    #[test]
    fn test_token_budget_calculation_4_to_1() {
        let ctx = WorkflowContext::with_token_budget(128_000, 4.0);
        assert_eq!(ctx.max_input_tokens(), 102_400); // 128k * (4/5) = 102.4k
        assert_eq!(ctx.max_output_tokens(), 25_600); // 128k * (1/5) = 25.6k
    }

    #[test]
    fn test_token_budget_calculation_1_to_1() {
        let ctx = WorkflowContext::with_token_budget(100_000, 1.0);
        assert_eq!(ctx.max_input_tokens(), 50_000); // 100k * (1/2) = 50k
        assert_eq!(ctx.max_output_tokens(), 50_000); // 100k * (1/2) = 50k
    }

    #[test]
    fn test_append_messages() {
        let mut ctx = WorkflowContext::new();
        let messages = vec![
            ChatMessage::system("test"),
            ChatMessage::user("hello"),
        ];
        ctx.append_messages(messages);
        assert_eq!(ctx.chat_history.len(), 2);
    }

    #[test]
    fn test_fork_creates_isolated_copy() {
        let mut ctx = WorkflowContext::new();
        ctx.append_messages(vec![ChatMessage::user("test")]);

        let forked = ctx.fork();
        assert_eq!(forked.chat_history.len(), 1);
        assert_ne!(forked.metadata.workflow_id, ctx.metadata.workflow_id);
        assert!(forked.metadata.workflow_id.contains("fork"));
    }

    #[tokio::test]
    async fn test_noop_manager_never_prunes() {
        let manager = NoOpManager::new();
        let messages = vec![ChatMessage::user("test")];

        assert!(!manager.should_prune(&messages, 100_000).await);

        let (pruned, freed) = manager.prune(messages.clone()).await.unwrap();
        assert_eq!(pruned.len(), messages.len());
        assert_eq!(freed, 0);
    }

    #[test]
    fn test_noop_manager_token_estimation() {
        let manager = NoOpManager::new();
        let messages = vec![
            ChatMessage::user("test"), // 4 chars = ~1 token
            ChatMessage::assistant("hello world"), // 11 chars = ~2 tokens
        ];
        let tokens = manager.estimate_tokens(&messages);
        assert_eq!(tokens, 3); // (4 + 11) / 4 = 3
    }
}
