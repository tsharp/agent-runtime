//! Context management strategies for keeping chat history within token budgets.

mod message_type;
mod sliding_window;
mod summarization;
mod token_budget;

use crate::llm::types::ChatMessage;

pub use message_type::MessageTypeManager;
pub use sliding_window::SlidingWindowManager;
pub use summarization::SummarizationManager;
pub use token_budget::TokenBudgetManager;

/// Simple token estimation helper used by all strategies
pub(crate) fn estimate_tokens_simple(messages: &[ChatMessage]) -> usize {
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
