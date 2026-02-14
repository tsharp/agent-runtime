#[cfg(test)]
mod tests {
    use crate::agent::{Agent, AgentConfig};
    use crate::types::{AgentInput, AgentInputMetadata};
    use serde_json::json;

    #[test]
    fn test_agent_config_builder() {
        let config = AgentConfig::builder("test_agent")
            .system_prompt("You are a test agent")
            .build();

        assert_eq!(config.name, "test_agent");
        assert_eq!(config.system_prompt, "You are a test agent");
        assert!(config.tools.is_none());
        assert_eq!(config.max_tool_iterations, 10);
    }

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig::builder("test_agent")
            .system_prompt("Test prompt")
            .build();

        let agent = Agent::new(config);
        assert_eq!(agent.name(), "test_agent");
        assert_eq!(agent.config().system_prompt, "Test prompt");
    }

    #[tokio::test]
    async fn test_agent_execute_without_llm() {
        let config = AgentConfig::builder("mock_agent")
            .system_prompt("Mock prompt")
            .build();

        let agent = Agent::new(config);

        let input = AgentInput {
            data: json!({"test": "data"}),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
        };

        let result = agent.execute(&input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.metadata.agent_name, "mock_agent");
        assert!(output.metadata.execution_time_ms < 10000); // Should complete quickly

        // Should be mock execution
        assert_eq!(output.data["agent"], "mock_agent");
        assert_eq!(output.data["system_prompt"], "Mock prompt");
    }

    #[test]
    fn test_agent_config_debug() {
        let config = AgentConfig::builder("debug_agent")
            .system_prompt("Debug prompt")
            .build();

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("debug_agent"));
        assert!(debug_str.contains("None"));
    }
}
