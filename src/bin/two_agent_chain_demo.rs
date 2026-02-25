//! Two-agent chain demo using a real LM Studio client.
//!
//! Agent 1 (Drafter)  – writes a short paragraph on the given topic.
//! Agent 2 (Critic)   – reviews the draft and suggests one concrete improvement.
//!
//! Both agents share a workflow chat history so the Critic sees what the
//! Drafter wrote without any extra wiring.

use agent_runtime::llm::LlamaClient;
use agent_runtime::{Agent, AgentConfig, AgentStep, Runtime, Workflow, WorkflowContext};
use chrono::Local;
use std::sync::Arc;
use tokio::task;

// ── configure these to match your LM Studio setup ──────────────────────────
const BASE_URL: &str = "http://localhost:1234";
const MODEL: &str = "zai-org/glm-4.6v-flash";

// The topic both agents will work on
const TOPIC: &str = "the importance of observability in distributed systems";
// ───────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[{}] === Two-Agent Chain Demo ===", ts());
    println!("  LM Studio : {}", BASE_URL);
    println!("  Model     : {}", MODEL);
    println!("  Topic     : {}\n", TOPIC);

    // Shared LM Studio client
    let client = Arc::new(LlamaClient::new(BASE_URL, MODEL));

    // ── Agent 1: Drafter ──────────────────────────────────────────────────
    let drafter = Agent::new(
        AgentConfig::builder("drafter")
            .system_prompt(
                "You are a technical writer. When given a topic, write a clear and \
                 concise paragraph (3-5 sentences) suitable for a developer audience.",
            )
            .build(),
    )
    .with_llm_client(client.clone());

    // ── Agent 2: Critic ───────────────────────────────────────────────────
    let critic = Agent::new(
        AgentConfig::builder("critic")
            .system_prompt(
                "You are a senior technical editor. Review the draft paragraph that \
                 was just written and suggest exactly one specific, actionable improvement \
                 to make it clearer or more precise. Be brief.",
            )
            .build(),
    )
    .with_llm_client(client.clone());

    // ── Shared workflow context (passes chat history between steps) ────────
    let context = WorkflowContext::with_token_budget(16_000, 3.0);

    // ── Build workflow ─────────────────────────────────────────────────────
    let workflow = Workflow::builder()
        .name("two_agent_chain".to_string())
        .with_restored_context(context)
        .initial_input(serde_json::json!(TOPIC))
        .step(Box::new(AgentStep::from_agent(
            drafter,
            "Drafter".to_string(),
        )))
        .step(Box::new(AgentStep::from_agent(
            critic,
            "Critic".to_string(),
        )))
        .build();

    let context_ref = workflow.context().unwrap().clone();

    // ── Runtime + event monitor ────────────────────────────────────────────
    let runtime = Runtime::new();
    let mut events = runtime.event_stream().subscribe();

    let monitor = task::spawn(async move {
        use agent_runtime::event::{EventScope, EventType};

        while let Ok(event) = events.recv().await {
            match (&event.scope, &event.event_type) {
                (EventScope::Agent, EventType::Started) => {
                    let name = event
                        .data
                        .get("agent")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&event.component_id);
                    println!("\n[{}] 🤖 {} starting...", ts(), name);
                }
                (EventScope::LlmRequest, EventType::Progress) => {
                    if let Some(chunk) = event.data.get("chunk").and_then(|v| v.as_str()) {
                        print!("{}", chunk);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                }
                (EventScope::LlmRequest, EventType::Completed) => {
                    println!(); // newline after streamed output
                }
                (EventScope::Agent, EventType::Completed) => {
                    let name = &event.component_id;
                    let ms = event
                        .data
                        .get("execution_time_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    println!("[{}] ✅ {} finished in {}ms", ts(), name, ms);
                }
                (EventScope::Workflow, EventType::Completed) => {
                    println!("\n[{}] 🏁 Workflow complete", ts());
                    break;
                }
                (EventScope::Workflow, EventType::Failed) => {
                    println!("\n[{}] ❌ Workflow failed", ts());
                    break;
                }
                _ => {}
            }
        }
    });

    // ── Execute ────────────────────────────────────────────────────────────
    println!("[{}] 🚀 Executing workflow...", ts());
    let run = runtime.execute(workflow).await;
    monitor.await.ok();

    // ── Final chat history ─────────────────────────────────────────────────
    println!("\n{}", "─".repeat(60));
    println!("[{}] 📜 Full conversation history:", ts());
    let ctx = context_ref.read().unwrap();
    for msg in ctx.history() {
        println!("\n[{:?}]\n{}", msg.role, msg.content);
    }

    println!("\n[{}] Workflow state: {:?}", ts(), run.state);
    Ok(())
}

fn ts() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}
