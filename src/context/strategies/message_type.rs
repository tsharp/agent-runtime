use super::estimate_tokens_simple;
use crate::context::{ContextError, ContextManager};
use crate::llm::types::{ChatMessage, Role};
use async_trait::async_trait;

/// Message type-based context manager that prioritizes messages by type
/// Keeps system messages, recent user/assistant pairs, and prunes old tool calls
pub struct MessageTypeManager {
    /// Maximum messages to keep
    pub(super) max_messages: usize,

    /// Number of recent user/assistant pairs to always keep
    pub(super) keep_recent_pairs: usize,
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

        while i > 0 && pairs_found < keep_pairs {
            i -= 1;

            if matches!(history[i].role, Role::User | Role::Assistant) {
                pair_indices.push(i);

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

    async fn prune(
        &self,
        history: Vec<ChatMessage>,
    ) -> Result<(Vec<ChatMessage>, usize), ContextError> {
        if history.len() <= self.max_messages {
            return Ok((history, 0));
        }

        let original_len = history.len();

        let system_indices: Vec<usize> = history
            .iter()
            .enumerate()
            .filter(|(_, msg)| msg.role == Role::System)
            .map(|(i, _)| i)
            .collect();

        let recent_pair_indices = Self::extract_recent_pairs(&history, self.keep_recent_pairs);

        let mut protected: std::collections::HashSet<usize> = system_indices.into_iter().collect();
        protected.extend(recent_pair_indices);

        let mut protected_vec: Vec<usize> = protected.iter().copied().collect();
        protected_vec.sort_unstable();

        let mut new_history = Vec::new();
        for &idx in &protected_vec {
            if idx < history.len() {
                new_history.push(history[idx].clone());
            }
        }

        if new_history.len() > self.max_messages {
            new_history.sort_by_key(Self::classify_message);
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

#[cfg(test)]
mod tests {
    use super::*;

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

        assert!(pruned.len() <= 10);
        assert!(removed > 0);
        assert!(pruned.iter().any(|m| m.role == Role::System));
        assert!(pruned.iter().any(|m| m.content == "Recent assistant 2"));
    }

    #[tokio::test]
    async fn test_message_type_manager_should_prune() {
        let manager = MessageTypeManager::new(5, 2);

        let short_history = vec![ChatMessage::user("msg1"), ChatMessage::assistant("resp1")];

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
}
