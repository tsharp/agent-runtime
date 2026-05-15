use crate::context::{ContextError, ContextManager};
use crate::llm::types::{ChatMessage, Role};
use async_trait::async_trait;

/// Sliding window context manager that keeps last N messages
pub struct SlidingWindowManager {
    /// Maximum number of messages to keep
    pub(super) max_messages: usize,

    /// Minimum messages to keep (typically system + 1 pair)
    pub(super) min_messages: usize,
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
            return Ok((history, 0));
        }

        let initial_count = history.len();

        let system_count = history
            .iter()
            .take_while(|msg| msg.role == Role::System)
            .count();

        let messages_to_keep = self.max_messages.saturating_sub(system_count);

        let system_messages: Vec<_> = history.drain(..system_count).collect();
        let remaining_len = history.len();

        let keep_from_index = remaining_len.saturating_sub(messages_to_keep);
        let mut kept_messages: Vec<_> = history.drain(keep_from_index..).collect();

        let mut pruned = system_messages;
        pruned.append(&mut kept_messages);

        let removed_count = initial_count - pruned.len();

        Ok((pruned, removed_count))
    }

    fn estimate_tokens(&self, messages: &[ChatMessage]) -> usize {
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
    fn test_sliding_window_creation() {
        let manager = SlidingWindowManager::new(10);
        assert_eq!(manager.max_messages, 10);
        assert_eq!(manager.min_messages, 3);
    }

    #[tokio::test]
    async fn test_sliding_window_should_prune() {
        let manager = SlidingWindowManager::new(5);
        let short_history = vec![ChatMessage::user("msg1"), ChatMessage::assistant("resp1")];
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

        assert_eq!(pruned.len(), 4);
        assert_eq!(pruned[0].role, Role::System);
        assert_eq!(pruned[pruned.len() - 1].content, "Recent resp");
        assert_eq!(removed, 3);
    }
}
