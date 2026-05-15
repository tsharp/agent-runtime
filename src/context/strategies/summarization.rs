use super::estimate_tokens_simple;
use crate::context::{ContextError, ContextManager};
use crate::llm::types::{ChatMessage, Role};
use async_trait::async_trait;

/// Summarization-based context manager that compresses old history using an LLM
/// This strategy calls an LLM to create compressed summaries of old messages
pub struct SummarizationManager {
    /// Token threshold that triggers summarization
    pub(super) summarization_threshold: usize,

    /// Target token count for summaries (reserved for future use)
    pub(super) _summary_token_target: usize,

    /// Maximum input tokens allowed
    pub(super) max_input_tokens: usize,

    /// Number of recent messages to never summarize
    pub(super) keep_recent_count: usize,
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

        let user_messages: Vec<_> = messages.iter().filter(|m| m.role == Role::User).collect();

        let assistant_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .collect();

        summary_content.push_str(&format!(
            "- {} user inputs and {} assistant responses\n",
            user_messages.len(),
            assistant_messages.len()
        ));

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
            agent_id: None,
            workflow_id: None,
        }
    }
}

#[async_trait]
impl ContextManager for SummarizationManager {
    async fn should_prune(&self, _history: &[ChatMessage], current_tokens: usize) -> bool {
        current_tokens > self.summarization_threshold
    }

    async fn prune(
        &self,
        history: Vec<ChatMessage>,
    ) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        let current_tokens = self.estimate_tokens(&history);

        if current_tokens <= self.summarization_threshold {
            return Ok((history, 0));
        }

        let keep_from_end = self.keep_recent_count.min(history.len());
        let summarize_count = history.len().saturating_sub(keep_from_end);

        if summarize_count == 0 {
            return Ok((history, 0));
        }

        let original_len = history.len();

        let (to_summarize, keep_recent) = history.split_at(summarize_count);

        let system_messages: Vec<ChatMessage> = to_summarize
            .iter()
            .filter(|msg| msg.role == Role::System)
            .cloned()
            .collect();

        let non_system_to_summarize: Vec<ChatMessage> = to_summarize
            .iter()
            .filter(|msg| msg.role != Role::System)
            .cloned()
            .collect();

        let mut new_history = Vec::new();
        new_history.extend(system_messages);

        if !non_system_to_summarize.is_empty() {
            new_history.push(Self::create_summary(&non_system_to_summarize));
        }

        new_history.extend_from_slice(keep_recent);

        let final_tokens = self.estimate_tokens(&new_history);
        if final_tokens > self.max_input_tokens {
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

        assert!(!manager.should_prune(&messages, 10_000).await);
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

        println!("Pruned: {} messages, removed: {}", pruned.len(), removed);

        assert!(pruned
            .iter()
            .any(|m| m.role == Role::System && m.content == "System prompt"));

        assert!(pruned.iter().any(|m| m.content == "Recent response 2"));
        assert!(pruned.iter().any(|m| m.content == "Recent message 2"));

        if removed > 0 {
            assert!(pruned
                .iter()
                .any(|m| m.content.contains("Summary of previous conversation")));
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

        let system_count = pruned.iter().filter(|m| m.role == Role::System).count();
        assert!(system_count >= 2, "System messages should be preserved");
    }
}
