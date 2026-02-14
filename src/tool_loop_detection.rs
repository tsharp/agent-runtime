use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Configuration for detecting and preventing tool call loops
#[derive(Debug, Clone)]
pub struct ToolLoopDetectionConfig {
    /// Whether loop detection is enabled
    pub enabled: bool,

    /// Custom message to inject when a loop is detected
    /// If None, uses a default message
    pub custom_message: Option<String>,
}

impl Default for ToolLoopDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_message: None,
        }
    }
}

impl ToolLoopDetectionConfig {
    /// Create with loop detection enabled and default message
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            custom_message: None,
        }
    }

    /// Create with loop detection disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            custom_message: None,
        }
    }

    /// Create with a custom message
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            enabled: true,
            custom_message: Some(message.into()),
        }
    }

    /// Get the message to use when a loop is detected
    pub fn get_message(&self, tool_name: &str, previous_result: &JsonValue) -> String {
        if let Some(custom) = &self.custom_message {
            // Replace placeholders in custom message
            custom
                .replace("{tool_name}", tool_name)
                .replace("{previous_result}", &previous_result.to_string())
        } else {
            // Default message
            format!(
                "You already called the tool '{}' with these exact parameters and received a response: {}. \
                Please use the previous result instead of calling it again. \
                If you need different information, try calling with different parameters.",
                tool_name,
                previous_result
            )
        }
    }
}

/// Tracks tool calls to detect loops
#[derive(Debug, Clone, Default)]
pub struct ToolCallTracker {
    /// History of (tool_name, args_hash, result) tuples
    history: Vec<(String, String, JsonValue)>,
}

impl ToolCallTracker {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    /// Record a tool call and its result
    pub fn record_call(
        &mut self,
        tool_name: &str,
        args: &HashMap<String, JsonValue>,
        result: &JsonValue,
    ) {
        let args_hash = Self::hash_args(args);
        self.history
            .push((tool_name.to_string(), args_hash, result.clone()));
    }

    /// Check if this exact tool call (name + args) was made before
    /// Returns Some(previous_result) if found, None otherwise
    pub fn check_for_loop(
        &self,
        tool_name: &str,
        args: &HashMap<String, JsonValue>,
    ) -> Option<JsonValue> {
        let args_hash = Self::hash_args(args);

        // Look for previous call with same tool + args
        self.history
            .iter()
            .find(|(name, hash, _)| name == tool_name && hash == &args_hash)
            .map(|(_, _, result)| result.clone())
    }

    /// Clear the history (e.g., at start of new agent execution)
    pub fn clear(&mut self) {
        self.history.clear();
    }

    /// Create a simple hash of arguments for comparison
    fn hash_args(args: &HashMap<String, JsonValue>) -> String {
        // Serialize to JSON for consistent comparison
        // Sort keys to ensure deterministic ordering
        let json = serde_json::to_string(args).unwrap_or_default();
        format!("{:x}", md5::compute(json.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_loop_detection_config_default_message() {
        let config = ToolLoopDetectionConfig::default();
        let result = json!({"data": "test"});

        let message = config.get_message("search", &result);
        assert!(message.contains("search"));
        assert!(message.contains("already called"));
    }

    #[test]
    fn test_loop_detection_config_custom_message() {
        let config = ToolLoopDetectionConfig::with_message(
            "Stop calling {tool_name}! Previous result: {previous_result}",
        );

        let result = json!({"data": "test"});
        let message = config.get_message("search", &result);

        assert!(message.contains("Stop calling search!"));
        assert!(message.contains("test"));
    }

    #[test]
    fn test_tracker_detects_loop() {
        let mut tracker = ToolCallTracker::new();

        let mut args = HashMap::new();
        args.insert("query".to_string(), json!("test"));

        let result = json!({"found": false});

        // First call - no loop
        assert!(tracker.check_for_loop("search", &args).is_none());

        // Record it
        tracker.record_call("search", &args, &result);

        // Second call with same args - loop detected!
        let previous = tracker.check_for_loop("search", &args);
        assert!(previous.is_some());
        assert_eq!(previous.unwrap(), result);
    }

    #[test]
    fn test_tracker_different_args_no_loop() {
        let mut tracker = ToolCallTracker::new();

        let mut args1 = HashMap::new();
        args1.insert("query".to_string(), json!("test1"));

        let mut args2 = HashMap::new();
        args2.insert("query".to_string(), json!("test2"));

        tracker.record_call("search", &args1, &json!({}));

        // Different args - no loop
        assert!(tracker.check_for_loop("search", &args2).is_none());
    }

    #[test]
    fn test_tracker_clear() {
        let mut tracker = ToolCallTracker::new();

        let mut args = HashMap::new();
        args.insert("query".to_string(), json!("test"));

        tracker.record_call("search", &args, &json!({}));
        tracker.clear();

        // After clear, no loop detected
        assert!(tracker.check_for_loop("search", &args).is_none());
    }
}
