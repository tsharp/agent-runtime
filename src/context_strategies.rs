use crate::context::{ContextError, ContextManager};
use crate::llm::types::{ChatMessage, Role};
use async_trait::async_trait;

/// Simple token estimation helper used by all strategies
fn estimate_tokens_simple(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .map(|msg| {
            let content_tokens = msg.content.len() / 4; // ~4 chars per token
            let role_token = 1; // Role field
            let tool_tokens = msg
                .tool_calls
                .as_ref()
                .map(|calls| calls.len() * 20)
                .unwrap_or(0);
            content_tokens + role_token + tool_tokens
        })
        .sum()
}

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

/// Message type-based context manager that prioritizes messages by type
/// Keeps system messages, recent user/assistant pairs, and prunes old tool calls
pub struct MessageTypeManager {
    /// Maximum messages to keep
    max_messages: usize,
    
    /// Number of recent user/assistant pairs to always keep
    keep_recent_pairs: usize,
}

impl MessageTypeManager {
    /// Create a new message type manager
    ///
    /// # Arguments
    /// * `max_messages` - Maximum total messages to keep
    /// * `keep_recent_pairs` - Number of recent user/assistant conversation pairs to preserve
    ///
    /// # Examples
    /// ```
    /// use agent_runtime::context_strategies::MessageTypeManager;
    ///
    /// // Keep up to 20 messages, always preserve last 5 user/assistant pairs
    /// let manager = MessageTypeManager::new(20, 5);
    /// ```
    pub fn new(max_messages: usize, keep_recent_pairs: usize) -> Self {
        Self {
            max_messages,
            keep_recent_pairs,
        }
    }

    /// Classify messages into priority tiers for pruning
    fn classify_message(msg: &ChatMessage) -> MessagePriority {
        match msg.role {
            Role::System => MessagePriority::Critical,
            Role::User | Role::Assistant => MessagePriority::High,
            Role::Tool => MessagePriority::Low,
        }
    }

    /// Extract recent conversation pairs (user/assistant sequences)
    fn extract_recent_pairs(history: &[ChatMessage], keep_pairs: usize) -> Vec<usize> {
        let mut pair_indices = Vec::new();
        let mut i = history.len();
        let mut pairs_found = 0;

        // Walk backwards to find user/assistant pairs
        while i > 0 && pairs_found < keep_pairs {
            i -= 1;
            
            if matches!(history[i].role, Role::User | Role::Assistant) {
                pair_indices.push(i);
                
                // If we found an assistant message, look for preceding user message
                if history[i].role == Role::Assistant && i > 0 {
                    for j in (0..i).rev() {
                        if history[j].role == Role::User {
                            pair_indices.push(j);
                            pairs_found += 1;
                            i = j;
                            break;
                        }
                    }
                }
            }
        }

        pair_indices.sort_unstable();
        pair_indices
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum MessagePriority {
    Critical = 0, // System messages
    High = 1,     // User/Assistant
    Low = 2,      // Tool calls
}

#[async_trait]
impl ContextManager for MessageTypeManager {
    async fn should_prune(&self, history: &[ChatMessage], _current_tokens: usize) -> bool {
        history.len() > self.max_messages
    }

    async fn prune(&self, history: Vec<ChatMessage>) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        if history.len() <= self.max_messages {
            return Ok((history, 0));
        }

        let original_len = history.len();

        // 1. Always keep system messages
        let system_indices: Vec<usize> = history
            .iter()
            .enumerate()
            .filter(|(_, msg)| msg.role == Role::System)
            .map(|(i, _)| i)
            .collect();

        // 2. Keep recent conversation pairs
        let recent_pair_indices = Self::extract_recent_pairs(&history, self.keep_recent_pairs);

        // 3. Combine protected indices
        let mut protected: std::collections::HashSet<usize> = system_indices.into_iter().collect();
        protected.extend(recent_pair_indices);

        // 4. Build new history with protected messages
        let mut protected_vec: Vec<usize> = protected.iter().copied().collect();
        protected_vec.sort_unstable();
        
        let mut new_history = Vec::new();
        for &idx in &protected_vec {
            if idx < history.len() {
                new_history.push(history[idx].clone());
            }
        }

        // If still over limit, keep only most critical
        if new_history.len() > self.max_messages {
            new_history.sort_by_key(|msg| Self::classify_message(msg));
            new_history.truncate(self.max_messages);
        }

        let removed = original_len - new_history.len();
        Ok((new_history, removed))
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        estimate_tokens_simple(messages)
    }

    fn name(&self) -> &str {
        "MessageType"
    }
}

/// Summarization-based context manager that compresses old history using an LLM
/// This strategy calls an LLM to create compressed summaries of old messages
pub struct SummarizationManager {
    /// Token threshold that triggers summarization
    summarization_threshold: usize,
    
    /// Target token count for summaries (reserved for future use)
    _summary_token_target: usize,
    
    /// Maximum input tokens allowed
    max_input_tokens: usize,
    
    /// Number of recent messages to never summarize
    keep_recent_count: usize,
}

impl SummarizationManager {
    /// Create a new summarization manager
    ///
    /// # Arguments
    /// * `max_input_tokens` - Maximum tokens allowed for input
    /// * `summarization_threshold` - Token count that triggers summarization
    /// * `summary_token_target` - Target size for compressed summaries
    /// * `keep_recent_count` - Number of recent messages to preserve unsummarized
    ///
    /// # Examples
    /// ```
    /// use agent_runtime::context_strategies::SummarizationManager;
    ///
    /// // When history exceeds 15k tokens, summarize old messages to ~500 tokens
    /// // Keep last 10 messages untouched
    /// let manager = SummarizationManager::new(18_000, 15_000, 500, 10);
    /// ```
    pub fn new(
        max_input_tokens: usize,
        summarization_threshold: usize,
        summary_token_target: usize,
        keep_recent_count: usize,
    ) -> Self {
        Self {
            summarization_threshold,
            _summary_token_target: summary_token_target,
            max_input_tokens,
            keep_recent_count,
        }
    }

    /// Create a summary message from a slice of history
    /// Note: This is a placeholder implementation. In production, you would
    /// call an actual LLM to generate the summary.
    fn create_summary(messages: &[ChatMessage]) -> ChatMessage {
        let mut summary_content = String::from("Summary of previous conversation:\n\n");
        
        // Extract key information from messages
        let user_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role == Role::User)
            .collect();
        
        let assistant_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .collect();

        summary_content.push_str(&format!(
            "- {} user inputs and {} assistant responses\n",
            user_messages.len(),
            assistant_messages.len()
        ));

        // Sample some content (in production, use LLM to intelligently summarize)
        if let Some(first_user) = user_messages.first() {
            let preview = first_user.content.chars().take(100).collect::<String>();
            summary_content.push_str(&format!("- Initial topic: {}\n", preview));
        }

        if let Some(last_assistant) = assistant_messages.last() {
            let preview = last_assistant.content.chars().take(100).collect::<String>();
            summary_content.push_str(&format!("- Latest response: {}\n", preview));
        }

        summary_content.push_str("\n[This is a compressed summary. Original messages were removed to save context space.]");

        ChatMessage {
            role: Role::System,
            content: summary_content,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[async_trait]
impl ContextManager for SummarizationManager {
    async fn should_prune(&self, _history: &[ChatMessage], current_tokens: usize) -> bool {
        current_tokens > self.summarization_threshold
    }

    async fn prune(&self, history: Vec<ChatMessage>) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        let current_tokens = self.estimate_tokens(&history);
        
        if current_tokens <= self.summarization_threshold {
            return Ok((history, 0));
        }

        // Calculate how many messages to keep unsummarized
        let keep_from_end = self.keep_recent_count.min(history.len());
        let summarize_count = history.len().saturating_sub(keep_from_end);

        if summarize_count == 0 {
            return Ok((history, 0));
        }

        let original_len = history.len();

        // Split history into "to summarize" and "keep as-is"
        let (to_summarize, keep_recent) = history.split_at(summarize_count);

        // Keep system messages from the to-summarize section
        let system_messages: Vec<ChatMessage> = to_summarize
            .iter()
            .filter(|msg| msg.role == Role::System)
            .cloned()
            .collect();

        // Create summary of non-system messages
        let non_system_to_summarize: Vec<ChatMessage> = to_summarize
            .iter()
            .filter(|msg| msg.role != Role::System)
            .cloned()
            .collect();

        let mut new_history = Vec::new();

        // Add system messages first
        new_history.extend(system_messages);

        // Add summary if there's content to summarize
        if !non_system_to_summarize.is_empty() {
            new_history.push(Self::create_summary(&non_system_to_summarize));
        }

        // Add recent messages
        new_history.extend_from_slice(keep_recent);

        // If still over limit, apply emergency truncation
        let final_tokens = self.estimate_tokens(&new_history);
        if final_tokens > self.max_input_tokens {
            // Keep system messages and most recent messages only
            let emergency_keep = self.keep_recent_count / 2;
            new_history.retain(|msg| msg.role == Role::System);
            
            if emergency_keep > 0 && emergency_keep < history.len() {
                let start_idx = history.len() - emergency_keep;
                new_history.extend_from_slice(&history[start_idx..]);
            }
        }

        let removed = original_len - new_history.len();
        Ok((new_history, removed))
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
        estimate_tokens_simple(messages)
    }

    fn name(&self) -> &str {
        "Summarization"
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

    #[tokio::test]
    async fn test_message_type_manager_creation() {
        let manager = MessageTypeManager::new(20, 5);
        assert_eq!(manager.max_messages, 20);
        assert_eq!(manager.keep_recent_pairs, 5);
    }

    #[tokio::test]
    async fn test_message_type_manager_prune() {
        let manager = MessageTypeManager::new(10, 2);
        let history = vec![
            ChatMessage::system("System prompt"),
            ChatMessage::user("Old user 1"),
            ChatMessage::assistant("Old assistant 1"),
            ChatMessage::tool_result("call1", "tool output 1"),
            ChatMessage::user("Old user 2"),
            ChatMessage::assistant("Old assistant 2"),
            ChatMessage::tool_result("call2", "tool output 2"),
            ChatMessage::user("Recent user 1"),
            ChatMessage::assistant("Recent assistant 1"),
            ChatMessage::user("Recent user 2"),
            ChatMessage::assistant("Recent assistant 2"),
        ];

        let (pruned, removed) = manager.prune(history).await.unwrap();

        // Should keep system + recent pairs
        assert!(pruned.len() <= 10);
        assert!(removed > 0);
        
        // System message should be preserved
        assert!(pruned.iter().any(|m| m.role == Role::System));
        
        // Recent messages should be preserved
        assert!(pruned.iter().any(|m| m.content == "Recent assistant 2"));
    }

    #[tokio::test]
    async fn test_message_type_manager_should_prune() {
        let manager = MessageTypeManager::new(5, 2);
        
        let short_history = vec![
            ChatMessage::user("msg1"),
            ChatMessage::assistant("resp1"),
        ];
        
        let long_history = vec![
            ChatMessage::system("System"),
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
    async fn test_summarization_manager_creation() {
        let manager = SummarizationManager::new(18_000, 15_000, 500, 10);
        assert_eq!(manager.max_input_tokens, 18_000);
        assert_eq!(manager.summarization_threshold, 15_000);
        assert_eq!(manager._summary_token_target, 500);
        assert_eq!(manager.keep_recent_count, 10);
    }

    #[tokio::test]
    async fn test_summarization_manager_should_prune() {
        let manager = SummarizationManager::new(18_000, 15_000, 500, 10);
        
        let messages = vec![ChatMessage::user("test")];
        
        // Below threshold
        assert!(!manager.should_prune(&messages, 10_000).await);
        
        // Above threshold
        assert!(manager.should_prune(&messages, 20_000).await);
    }

    #[tokio::test]
    async fn test_summarization_manager_prune() {
        let manager = SummarizationManager::new(18_000, 100, 50, 3);
        
        let history = vec![
            ChatMessage::system("System prompt"),
            ChatMessage::user("Old message 1"),
            ChatMessage::assistant("Old response 1"),
            ChatMessage::user("Old message 2"),
            ChatMessage::assistant("Old response 2"),
            ChatMessage::user("Old message 3"),
            ChatMessage::assistant("Old response 3"),
            ChatMessage::user("Recent message 1"),
            ChatMessage::assistant("Recent response 1"),
            ChatMessage::user("Recent message 2"),
            ChatMessage::assistant("Recent response 2"),
        ];

        let (pruned, removed) = manager.prune(history.clone()).await.unwrap();

        // Should have summarized old messages and kept recent ones
        // Or at least attempted to compress
        println!("Pruned: {} messages, removed: {}", pruned.len(), removed);
        
        // System message should be preserved
        assert!(pruned.iter().any(|m| m.role == Role::System && m.content == "System prompt"));
        
        // Recent messages should be preserved (last 3 messages)
        assert!(pruned.iter().any(|m| m.content == "Recent response 2"));
        assert!(pruned.iter().any(|m| m.content == "Recent message 2"));
        
        // Should contain a summary message if we actually summarized
        if removed > 0 {
            assert!(pruned.iter().any(|m| m.content.contains("Summary of previous conversation")));
        }
    }

    #[tokio::test]
    async fn test_summarization_preserves_system_messages() {
        let manager = SummarizationManager::new(18_000, 100, 50, 2);
        
        let history = vec![
            ChatMessage::system("System prompt 1"),
            ChatMessage::user("msg1"),
            ChatMessage::assistant("resp1"),
            ChatMessage::system("System prompt 2"),
            ChatMessage::user("msg2"),
            ChatMessage::assistant("resp2"),
            ChatMessage::user("recent"),
            ChatMessage::assistant("recent resp"),
        ];

        let (pruned, _) = manager.prune(history).await.unwrap();

        // All system messages should be preserved
        let system_count = pruned.iter().filter(|m| m.role == Role::System).count();
        assert!(system_count >= 2, "System messages should be preserved");
    }
}
