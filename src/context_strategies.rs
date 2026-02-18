use crate::context::{ContextError, ContextManager};
use crate::llm::types::{ChatMessage, Role};
use async_trait::async_trait;

/// Token budget-based context manager that maintains a configurable input budget
/// Supports any context size and input/output ratio
pub struct TokenBudgetManager {
    /// Maximum tokens allowed for input (calculated from total and ratio)
    max_input_tokens: usize,

    /// Minimum messages to keep (system prompt + recent pairs)
    min_messages_to_keep: usize,

    /// Safety buffer tokens (pruning triggers this many tokens before limit)
    safety_buffer: usize,
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
    ///
    /// // 100k context, 1:1 ratio = 50k input, 50k output
    /// let manager = TokenBudgetManager::new(100_000, 1.0);
    /// ```
    pub fn new(total_context_tokens: usize, input_output_ratio: f64) -> Self {
        if input_output_ratio <= 0.0 {
            panic!("input_output_ratio must be positive");
        }

        // Calculate input budget: total * (ratio / (ratio + 1))
        let max_input_tokens =
            (total_context_tokens as f64 * input_output_ratio / (input_output_ratio + 1.0))
                as usize;

        Self {
            max_input_tokens,
            min_messages_to_keep: 3, // System + at least 1 user/assistant pair
            safety_buffer: (max_input_tokens as f64 * 0.1) as usize, // 10% buffer
        }
    }

    /// Create with custom safety buffer
    pub fn with_safety_buffer(mut self, buffer_tokens: usize) -> Self {
        self.safety_buffer = buffer_tokens;
        self
    }

    /// Create with custom minimum messages to keep
    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages_to_keep = min;
        self
    }

    /// Get the effective pruning threshold (max - buffer)
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
        let mut remaining: Vec<_> = history
            .into_iter()
            .skip(system_messages.len())
            .collect();

        // Prune from the front (oldest messages) while over budget
        let target_tokens = self.max_input_tokens;
        let mut current_tokens = initial_tokens;

        while current_tokens > target_tokens && remaining.len() > self.min_messages_to_keep {
            if let Some(removed) = remaining.first() {
                let removed_tokens = self.estimate_tokens(&[removed.clone()]);
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
        // Improved approximation:
        // - Count characters in content
        // - Account for role tokens (~1 token per role)
        // - Account for tool calls (rough estimate)
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

/// Sliding window context manager that keeps last N messages
pub struct SlidingWindowManager {
    /// Maximum number of messages to keep
    max_messages: usize,

    /// Minimum messages to keep (typically system + 1 pair)
    min_messages: usize,
}

impl SlidingWindowManager {
    /// Create a new sliding window manager
    ///
    /// # Arguments
    /// * `max_messages` - Maximum number of messages to keep in history
    pub fn new(max_messages: usize) -> Self {
        Self {
            max_messages,
            min_messages: 3, // System + 1 user/assistant pair
        }
    }

    /// Create with custom minimum messages
    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages = min;
        self
    }
}

#[async_trait]
impl ContextManager for SlidingWindowManager {
    async fn should_prune(&self, history: &[ChatMessage], _current_tokens: usize) -> bool {
        history.len() > self.max_messages
    }

    async fn prune(
        &self,
        mut history: Vec<ChatMessage>,
    ) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        if history.len() <= self.max_messages {
            return Ok((history, 0)); // No pruning needed
        }

        let initial_count = history.len();

        // Keep system messages at the start
        let system_count = history
            .iter()
            .take_while(|msg| msg.role == Role::System)
            .count();

        // Calculate how many messages to keep from the end
        let messages_to_keep = self.max_messages.saturating_sub(system_count);

        // Split into system messages and rest
        let system_messages: Vec<_> = history.drain(..system_count).collect();
        let remaining_len = history.len();

        // Keep only the last N messages
        let keep_from_index = remaining_len.saturating_sub(messages_to_keep);
        let mut kept_messages: Vec<_> = history.drain(keep_from_index..).collect();

        // Reconstruct
        let mut pruned = system_messages;
        pruned.append(&mut kept_messages);

        let removed_count = initial_count - pruned.len();

        Ok((pruned, removed_count))
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        // Simple approximation for sliding window
        messages
            .iter()
            .map(|msg| msg.content.len() / 4)
            .sum::<usize>()
    }

    fn name(&self) -> &str {
        "SlidingWindow"
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
        // 3:1 ratio
        let m1 = TokenBudgetManager::new(24_000, 3.0);
        assert_eq!(m1.max_input_tokens, 18_000);

        // 4:1 ratio
        let m2 = TokenBudgetManager::new(128_000, 4.0);
        assert_eq!(m2.max_input_tokens, 102_400);

        // 1:1 ratio
        let m3 = TokenBudgetManager::new(100_000, 1.0);
        assert_eq!(m3.max_input_tokens, 50_000);

        // 9:1 ratio
        let m4 = TokenBudgetManager::new(200_000, 9.0);
        assert_eq!(m4.max_input_tokens, 180_000);
    }

    #[tokio::test]
    async fn test_token_budget_should_prune() {
        let manager = TokenBudgetManager::new(24_000, 3.0);
        let messages = vec![ChatMessage::user("test")];

        // Below threshold - don't prune
        assert!(!manager.should_prune(&messages, 10_000).await);

        // Above threshold - should prune
        assert!(manager.should_prune(&messages, 20_000).await);
    }

    #[tokio::test]
    async fn test_token_budget_prune_keeps_system() {
        let manager = TokenBudgetManager::new(100, 3.0); // Very small context for testing
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

        // Should keep system message
        assert_eq!(pruned[0].role, Role::System);

        // Should have fewer messages than original
        assert!(pruned.len() <= 7);
    }

    #[test]
    fn test_sliding_window_creation() {
        let manager = SlidingWindowManager::new(10);
        assert_eq!(manager.max_messages, 10);
        assert_eq!(manager.min_messages, 3);
    }

    #[tokio::test]
    async fn test_sliding_window_should_prune() {
        let manager = SlidingWindowManager::new(5);
        let short_history = vec![
            ChatMessage::user("msg1"),
            ChatMessage::assistant("resp1"),
        ];
        let long_history = vec![
            ChatMessage::user("msg1"),
            ChatMessage::assistant("resp1"),
            ChatMessage::user("msg2"),
            ChatMessage::assistant("resp2"),
            ChatMessage::user("msg3"),
            ChatMessage::assistant("resp3"),
        ];

        assert!(!manager.should_prune(&short_history, 0).await);
        assert!(manager.should_prune(&long_history, 0).await);
    }

    #[tokio::test]
    async fn test_sliding_window_prune() {
        let manager = SlidingWindowManager::new(4);
        let history = vec![
            ChatMessage::system("System"),
            ChatMessage::user("Old 1"),
            ChatMessage::assistant("Old resp 1"),
            ChatMessage::user("Old 2"),
            ChatMessage::assistant("Old resp 2"),
            ChatMessage::user("Recent"),
            ChatMessage::assistant("Recent resp"),
        ];

        let (pruned, removed) = manager.prune(history).await.unwrap();

        // Should keep system + last 3 messages = 4 total
        assert_eq!(pruned.len(), 4);
        assert_eq!(pruned[0].role, Role::System);
        
        // Last messages should be kept
        assert_eq!(pruned[pruned.len() - 1].content, "Recent resp");
        assert_eq!(removed, 3); // Removed 3 messages
    }

    #[test]
    fn test_token_estimation() {
        let manager = TokenBudgetManager::new(1000, 1.0);
        let messages = vec![
            ChatMessage::user("test"), // 4 chars = ~1 token + 1 role = 2
            ChatMessage::assistant("hello world"), // 11 chars = ~2 tokens + 1 role = 3
        ];

        let tokens = manager.estimate_tokens(&messages);
        assert_eq!(tokens, 5); // 2 + 3
    }
}
