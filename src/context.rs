//! Context management: a token budget and a compactor that keeps the message
//! list within bounds by folding the oldest messages into a summary.

use crate::agent::Message;

/// A rough token estimate: ~4 characters per token.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

/// Tracks a token budget and compacts the message list when it grows too large.
#[derive(Debug, Clone)]
pub struct ContextManager {
    max_tokens: usize,
    /// Token count at which the oldest messages are folded into a summary.
    compact_threshold: usize,
}

impl ContextManager {
    pub fn new(max_tokens: usize) -> Self {
        let compact_threshold = (max_tokens as f64 * 0.8) as usize;
        Self {
            max_tokens,
            compact_threshold,
        }
    }

    /// Estimate the total tokens in a message list.
    pub fn estimate(&self, messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| estimate_tokens(&message_text(m)))
            .sum()
    }

    /// Whether the message list is over the hard budget.
    pub fn over_budget(&self, messages: &[Message]) -> bool {
        self.estimate(messages) > self.max_tokens
    }

    /// Compact the message list if it exceeds the threshold. Folds the oldest
    /// messages (excluding the system prompt) into a single summary message.
    /// Returns true if a compaction happened.
    pub fn compact(&self, messages: &mut Vec<Message>) -> bool {
        if self.estimate(messages) <= self.compact_threshold {
            return false;
        }
        // Keep the system prompt and the most recent half; fold the rest.
        let keep_from = (messages.len() / 2).max(1);
        let mut summary = String::from("[summary of earlier conversation]\n");
        for message in &messages[1..keep_from] {
            let text = message_text(message);
            // Bound each folded message so the summary is genuinely smaller than
            // what it replaces. A real model-based summarizer can replace this.
            let truncated: String = text.chars().take(20).collect();
            summary.push_str(&truncated);
            summary.push('\n');
        }
        // Replace the folded messages with the summary: keep the system prompt,
        // drop messages[1..keep_from], keep the tail.
        let tail = messages.split_off(keep_from);
        messages.truncate(1);
        messages.push(Message::System(summary));
        messages.extend(tail);
        true
    }
}

fn message_text(message: &Message) -> String {
    match message {
        Message::System(t) | Message::User(t) => t.clone(),
        Message::Assistant { content, .. } => content.clone(),
        Message::Tool { output, .. } => output.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_is_nonzero() {
        assert!(estimate_tokens("hello world this is a test") > 0);
    }

    #[test]
    fn compact_folds_old_messages() {
        let manager = ContextManager::new(50); // small budget so compaction triggers
        let mut messages = vec![Message::System("sys".into())];
        for i in 0..20 {
            messages.push(Message::User(format!(
                "message number {i} with some padding text"
            )));
        }
        let before_tokens = manager.estimate(&messages);
        let compacted = manager.compact(&mut messages);
        assert!(compacted);
        // A summary message was inserted and the token estimate dropped.
        assert!(messages
            .iter()
            .any(|m| matches!(m, Message::System(s) if s.contains("summary"))));
        assert!(manager.estimate(&messages) < before_tokens);
        // The system prompt is preserved at the front.
        assert!(matches!(messages[0], Message::System(_)));
    }
}
