use crate::context::{ContextError, ContextManager};
use crate::llm::types::{ChatMessage, Role};
use async_trait::async_trait;

/// Token budget-based context manager that maintains a configurable input budget
/// Supports any context size and input/output ratio
pub struct TokenBudgetManager {
    /// Maximum tokens allowed for input (calculated from total and ratio)
    pub(super) max_input_tokens: usize,

    /// Minimum messages to keep (system prompt + recent pairs)
    pub(super) min_messages_to_keep: usize,

    /// Safety buffer tokens (pruning triggers this many tokens before limit)
    pub(super) safety_buffer: usize,
}

impl TokenBudgetManager {
    /// Create a new token budget manager
    ///
    /// # Arguments
    /// * `total_context_tokens` - Total context window size (e.g., 24_000, 128_000)
    /// * `input_output_ratio` - Ratio of input to output tokens (e.g., 3.0 for 3:1)
    ///
    /// # Examples
    /// ```
    /// use agent_runtime::context_strategies::TokenBudgetManager;
    ///
    /// // 24k context, 3:1 ratio = 18k input, 6k output
    /// let manager = TokenBudgetManager::new(24_000, 3.0);
    ///
    /// // 128k context, 4:1 ratio = 102.4k input, 25.6k output
    /// let manager = TokenBudgetManager::new(128_000, 4.0);
    /// ```
    pub fn new(total_context_tokens: usize, input_output_ratio: f64) -> Self {
        let max_input = (total_context_tokens as f64 * input_output_ratio
            / (input_output_ratio + 1.0)) as usize;

        Self {
            max_input_tokens: max_input,
            min_messages_to_keep: 3,       // System + 1 user/assistant pair
            safety_buffer: max_input / 10, // 10% safety buffer
        }
    }

    /// Create with custom safety buffer
    pub fn with_safety_buffer(mut self, buffer: usize) -> Self {
        self.safety_buffer = buffer;
        self
    }

    /// Create with custom minimum messages
    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages_to_keep = min;
        self
    }

    /// Get the effective pruning threshold (max - safety buffer)
    pub fn pruning_threshold(&self) -> usize {
        self.max_input_tokens.saturating_sub(self.safety_buffer)
    }
}

#[async_trait]
impl ContextManager for TokenBudgetManager {
    async fn should_prune(&self, _history: &[ChatMessage], current_tokens: usize) -> bool {
        current_tokens > self.pruning_threshold()
    }

    async fn prune(
        &self,
        history: Vec<ChatMessage>,
    ) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        if history.len() <= self.min_messages_to_keep {
            return Ok((history, 0)); // Can't prune further
        }

        let initial_tokens = self.estimate_tokens(&history);

        // Always keep system messages at the start
        let system_messages: Vec<_> = history
            .iter()
            .take_while(|msg| msg.role == Role::System)
            .cloned()
            .collect();

        // Get messages after system messages
        let mut remaining: Vec<_> = history.into_iter().skip(system_messages.len()).collect();

        // Prune from the front (oldest messages) while over budget
        let target_tokens = self.max_input_tokens;
        let mut current_tokens = initial_tokens;

        while current_tokens > target_tokens && remaining.len() > self.min_messages_to_keep {
            if let Some(removed) = remaining.first() {
                let removed_tokens = self.estimate_tokens(std::slice::from_ref(removed));
                remaining.remove(0);
                current_tokens = current_tokens.saturating_sub(removed_tokens);
            } else {
                break;
            }
        }

        // Reconstruct: system messages + remaining messages
        let mut pruned = system_messages;
        pruned.extend(remaining);

        let final_tokens = self.estimate_tokens(&pruned);
        let tokens_freed = initial_tokens.saturating_sub(final_tokens);

        Ok((pruned, tokens_freed))
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        messages
            .iter()
            .map(|msg| {
                let content_tokens = msg.content.len() / 4; // ~4 chars per token
                let role_tokens = 1; // Role marker
                let tool_tokens = msg
                    .tool_calls
                    .as_ref()
                    .map(|calls| calls.len() * 20) // ~20 tokens per tool call
                    .unwrap_or(0);
                content_tokens + role_tokens + tool_tokens
            })
            .sum()
    }

    fn name(&self) -> &str {
        "TokenBudget"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_budget_manager_creation() {
        let manager = TokenBudgetManager::new(24_000, 3.0);
        assert_eq!(manager.max_input_tokens, 18_000);
        assert_eq!(manager.safety_buffer, 1_800); // 10% of 18k
        assert_eq!(manager.pruning_threshold(), 16_200); // 18k - 1.8k
    }

    #[test]
    fn test_token_budget_various_ratios() {
        let m1 = TokenBudgetManager::new(24_000, 3.0);
        assert_eq!(m1.max_input_tokens, 18_000);

        let m2 = TokenBudgetManager::new(128_000, 4.0);
        assert_eq!(m2.max_input_tokens, 102_400);

        let m3 = TokenBudgetManager::new(100_000, 1.0);
        assert_eq!(m3.max_input_tokens, 50_000);

        let m4 = TokenBudgetManager::new(200_000, 9.0);
        assert_eq!(m4.max_input_tokens, 180_000);
    }

    #[tokio::test]
    async fn test_token_budget_should_prune() {
        let manager = TokenBudgetManager::new(24_000, 3.0);
        let messages = vec![ChatMessage::user("test")];

        assert!(!manager.should_prune(&messages, 10_000).await);
        assert!(manager.should_prune(&messages, 20_000).await);
    }

    #[tokio::test]
    async fn test_token_budget_prune_keeps_system() {
        let manager = TokenBudgetManager::new(100, 3.0);
        let history = vec![
            ChatMessage::system("System prompt"),
            ChatMessage::user("Old message 1"),
            ChatMessage::assistant("Old response 1"),
            ChatMessage::user("Old message 2"),
            ChatMessage::assistant("Old response 2"),
            ChatMessage::user("Recent message"),
            ChatMessage::assistant("Recent response"),
        ];

        let (pruned, _tokens_freed) = manager.prune(history).await.unwrap();

        assert_eq!(pruned[0].role, Role::System);
        assert!(pruned.len() <= 7);
    }

    #[test]
    fn test_token_estimation() {
        let manager = TokenBudgetManager::new(1000, 1.0);
        let messages = vec![
            ChatMessage::user("test"),
            ChatMessage::assistant("hello world"),
        ];

        let tokens = manager.estimate_tokens(&messages);
        assert_eq!(tokens, 5);
    }
}
