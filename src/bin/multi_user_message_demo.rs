//! Demonstrates sending two consecutive user messages to a single agent.
//!
//! Simulates a scenario where a prior turn's context is already in the
//! chat history and a second question is appended before the agent responds —
//! both questions are answered in one shot.

use agent_runtime::llm::LlamaClient;
use agent_runtime::llm::types::ChatMessage;
use agent_runtime::types::{AgentInput, AgentInputMetadata};
use agent_runtime::{Agent, AgentConfig, Runtime};
use chrono::Local;
use std::sync::Arc;
use tokio::task;

const BASE_URL: &str = "http://localhost:1234";
const MODEL: &str = "zai-org/glm-4.6v-flash";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[{}] === Multi-User-Message Demo ===", ts());
    println!("  LM Studio : {}", BASE_URL);
    println!("  Model     : {}\n", MODEL);

    let client = Arc::new(LlamaClient::new(BASE_URL, MODEL));

    let agent = Agent::new(
        AgentConfig::builder("assistant")
            .system_prompt(
                "You are a knowledgeable assistant. Answer every question the user \
                 has asked, addressing each one clearly and in order.",
            )
            .strip_think_blocks(true)
            .build(),
    )
    .with_llm_client(client);

    // Two user messages with no assistant turn between them.
    // The agent receives both at once and is expected to address both.
    let history = vec![
        ChatMessage::user("What is the capital of France?"),
        ChatMessage::user("And what is the capital of Germany?"),
    ];

    println!("[{}] Sending two consecutive user messages:", ts());
    for (i, msg) in history.iter().enumerate() {
        println!("  [user {}] {}", i + 1, msg.content);
    }
    println!();

    // Runtime + event monitor for streaming output
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
                    println!("[{}] 🤖 {} >\n", ts(), name);
                }
                (EventScope::LlmRequest, EventType::Progress) => {
                    if let Some(chunk) = event.data.get("chunk").and_then(|v| v.as_str()) {
                        print!("{}", chunk);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                }
                (EventScope::LlmRequest, EventType::Completed) => {
                    println!();
                }
                (EventScope::Agent, EventType::Completed) => {
                    let ms = event
                        .data
                        .get("execution_time_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    println!("\n[{}] ✅ Completed in {}ms", ts(), ms);
                    break;
                }
                (EventScope::Agent, EventType::Failed) => {
                    println!("\n[{}] ❌ Agent failed: {:?}", ts(), event.message);
                    break;
                }
                _ => {}
            }
        }
    });

    let input = AgentInput {
        data: serde_json::Value::Null, // last message is already User — no extra turn needed
        metadata: AgentInputMetadata {
            step_index: 0,
            previous_agent: None,
        },
        chat_history: Some(history),
    };

    match agent
        .execute_with_events(input, Some(runtime.event_stream()))
        .await
    {
        Ok(output) => {
            monitor.await.ok();

            println!("\n{}", "─".repeat(60));
            println!("[{}] 📜 Final chat history with provenance:", ts());
            if let Some(hist) = &output.chat_history {
                for msg in hist {
                    let provenance = match (&msg.agent_id, &msg.workflow_id) {
                        (Some(a), Some(w)) => format!(" [agent={}, wf={}]", a, w),
                        (Some(a), None) => format!(" [agent={}]", a),
                        _ => String::new(),
                    };
                    println!("\n[{:?}{}]\n{}", msg.role, provenance, msg.content);
                }
            }
        }
        Err(e) => {
            monitor.await.ok();
            eprintln!("\n[{}] ❌ Error: {}", ts(), e);
        }
    }

    Ok(())
}

fn ts() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}
