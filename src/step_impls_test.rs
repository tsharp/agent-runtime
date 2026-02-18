#[cfg(test)]
mod tests {
    use crate::step::StepInputMetadata;
    use crate::step_impls::{AgentStep, TransformStep};
    use crate::{Agent, AgentConfig, Step, StepInput};
    use serde_json::json;

    #[tokio::test]
    async fn test_agent_step_execution() {
        let config = AgentConfig::builder("test_agent")
            .system_prompt("Test prompt")
            .build();

        let agent = Agent::new(config);
        let step = AgentStep::from_agent(agent, "test_step".to_string());

        assert_eq!(step.name(), "test_step");

        let input = StepInput {
            data: json!({"input": "data"}),
            metadata: StepInputMetadata {
                step_index: 0,
                previous_step: None,
                workflow_id: "wf_123".to_string(),
            },
            workflow_context: None,
        };

        let result = step.execute(input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.metadata.step_name, "test_step");
    }

    #[tokio::test]
    async fn test_transform_step() {
        let transform = TransformStep::new("double".to_string(), |data| {
            if let Some(value) = data.as_i64() {
                json!(value * 2)
            } else {
                data
            }
        });

        assert_eq!(transform.name(), "double");

        let input = StepInput {
            data: json!(5),
            metadata: StepInputMetadata {
                step_index: 0,
                previous_step: None,
                workflow_id: "wf_123".to_string(),
            },
            workflow_context: None,
        };

        let result = transform.execute(input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.data, json!(10));
        assert_eq!(output.metadata.step_name, "double");
    }

    #[tokio::test]
    async fn test_transform_step_complex() {
        let transform = TransformStep::new("extract_field".to_string(), |data| {
            data.get("message").cloned().unwrap_or(json!("default"))
        });

        let input = StepInput {
            data: json!({"message": "extracted"}),
            metadata: StepInputMetadata {
                step_index: 0,
                previous_step: None,
                workflow_id: "wf_123".to_string(),
            },
            workflow_context: None,
        };

        let result = transform.execute(input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.data, json!("extracted"));
    }

    #[test]
    fn test_step_type() {
        use crate::StepType;

        let config = AgentConfig::builder("test").system_prompt("Test").build();

        let agent = Agent::new(config);
        let step = AgentStep::from_agent(agent, "step".to_string());

        assert_eq!(step.step_type(), StepType::Agent);

        let transform = TransformStep::new("transform".to_string(), |d| d);
        assert_eq!(transform.step_type(), StepType::Transform);
    }
}
