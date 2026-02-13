#[cfg(test)]
mod tests {
    use crate::{Agent, AgentConfig, AgentStep, Runtime, Workflow, WorkflowState};
    use serde_json::json;

    #[test]
    fn test_workflow_builder() {
        let agent = Agent::new(
            AgentConfig::builder("test_agent")
                .system_prompt("Test")
                .build(),
        );

        let workflow = Workflow::builder()
            .step(Box::new(AgentStep::from_agent(agent, "step1".to_string())))
            .initial_input(json!({"test": "data"}))
            .build();

        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.initial_input, json!({"test": "data"}));
    }

    #[test]
    fn test_workflow_multi_step() {
        let agent1 = Agent::new(
            AgentConfig::builder("agent1")
                .system_prompt("First")
                .build(),
        );

        let agent2 = Agent::new(
            AgentConfig::builder("agent2")
                .system_prompt("Second")
                .build(),
        );

        let workflow = Workflow::builder()
            .step(Box::new(AgentStep::from_agent(agent1, "step1".to_string())))
            .step(Box::new(AgentStep::from_agent(agent2, "step2".to_string())))
            .initial_input(json!({"start": true}))
            .build();

        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_workflow_mermaid_generation() {
        let agent = Agent::new(
            AgentConfig::builder("test_agent")
                .system_prompt("Test")
                .build(),
        );

        let workflow = Workflow::builder()
            .step(Box::new(AgentStep::from_agent(
                agent,
                "test_step".to_string(),
            )))
            .initial_input(json!({}))
            .build();

        let mermaid = workflow.to_mermaid();
        assert!(mermaid.contains("flowchart TD"));
        assert!(mermaid.contains("Start"));
        assert!(mermaid.contains("End"));
        assert!(mermaid.contains("test_step"));
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let agent = Agent::new(
            AgentConfig::builder("test_agent")
                .system_prompt("Test agent")
                .build(),
        );

        let workflow = Workflow::builder()
            .step(Box::new(AgentStep::from_agent(agent, "step1".to_string())))
            .initial_input(json!({"message": "test"}))
            .build();

        let runtime = Runtime::new();
        let result = runtime.execute(workflow).await;

        assert_eq!(result.steps.len(), 1);
        assert!(result.final_output.is_some());
    }

    #[test]
    fn test_workflow_state() {
        let state = WorkflowState::Pending;
        assert_eq!(state, WorkflowState::Pending);

        let state = WorkflowState::Running;
        assert_eq!(state, WorkflowState::Running);

        let state = WorkflowState::Completed;
        assert_eq!(state, WorkflowState::Completed);

        let state = WorkflowState::Failed;
        assert_eq!(state, WorkflowState::Failed);
    }
}
