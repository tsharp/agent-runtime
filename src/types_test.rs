#[cfg(test)]
mod tests {
    use crate::types::*;
    use crate::{StepType, StepError, StepInput, StepOutput};
    use crate::step::{StepInputMetadata, StepOutputMetadata};
    use serde_json::json;

    #[test]
    fn test_agent_input_creation() {
        let input = AgentInput {
            data: json!({"message": "test"}),
            metadata: AgentInputMetadata {
                step_index: 0,
                previous_agent: None,
            },
        };

        assert_eq!(input.metadata.step_index, 0);
        assert_eq!(input.metadata.previous_agent, None);
        assert_eq!(input.data["message"], "test");
    }

    #[test]
    fn test_agent_output_creation() {
        let output = AgentOutput {
            data: json!({"result": "success"}),
            metadata: AgentOutputMetadata {
                agent_name: "test_agent".to_string(),
                execution_time_ms: 100,
                tool_calls_count: 2,
            },
        };

        assert_eq!(output.metadata.agent_name, "test_agent");
        assert_eq!(output.metadata.execution_time_ms, 100);
        assert_eq!(output.metadata.tool_calls_count, 2);
        assert_eq!(output.data["result"], "success");
    }

    #[test]
    fn test_agent_error_display() {
        let error = AgentError::ToolError("Tool failed".to_string());
        assert_eq!(error.to_string(), "Tool execution failed: Tool failed");

        let error = AgentError::InvalidInput("Bad input".to_string());
        assert_eq!(error.to_string(), "Invalid input: Bad input");

        let error = AgentError::ExecutionError("Execution failed".to_string());
        assert_eq!(error.to_string(), "Execution failed: Execution failed");
    }

    #[test]
    fn test_step_input_creation() {
        let input = StepInput {
            data: json!({"step_data": "value"}),
            metadata: StepInputMetadata {
                step_index: 1,
                previous_step: Some("previous".to_string()),
                workflow_id: "wf_123".to_string(),
            },
        };

        assert_eq!(input.metadata.step_index, 1);
        assert_eq!(input.metadata.previous_step, Some("previous".to_string()));
        assert_eq!(input.metadata.workflow_id, "wf_123");
    }

    #[test]
    fn test_step_output_creation() {
        let output = StepOutput {
            data: json!({"output": "data"}),
            metadata: StepOutputMetadata {
                step_name: "step1".to_string(),
                step_type: StepType::Agent,
                execution_time_ms: 500,
            },
        };

        assert_eq!(output.metadata.step_name, "step1");
        assert_eq!(output.metadata.step_type, StepType::Agent);
        assert_eq!(output.metadata.execution_time_ms, 500);
    }

    #[test]
    fn test_step_type_serialization() {
        let step_type = StepType::Agent;
        let json = serde_json::to_string(&step_type).unwrap();
        assert_eq!(json, "\"agent\"");

        let step_type = StepType::Transform;
        let json = serde_json::to_string(&step_type).unwrap();
        assert_eq!(json, "\"transform\"");

        let step_type = StepType::Conditional;
        let json = serde_json::to_string(&step_type).unwrap();
        assert_eq!(json, "\"conditional\"");
    }

    #[test]
    fn test_step_error_conversion() {
        let error = StepError::AgentError("Agent failed".to_string());
        assert_eq!(error.to_string(), "Agent error: Agent failed");

        let error = StepError::ExecutionFailed("Execution failed".to_string());
        assert_eq!(error.to_string(), "Execution failed: Execution failed");
    }
}
