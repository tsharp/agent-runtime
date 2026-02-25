//! Scene + Actor multi-agent demo.
//!
//! Two asymmetric agents interact in a text-adventure style loop:
//!
//!   World (narrator) — owns and evolves the scene state. Responds to the
//!                      Actor's actions by describing what happens next.
//!
//!   Actor (explorer) — observes the scene and decides what to do. Responds
//!                      to the World's descriptions with a concrete action.
//!
//! History alternates user/assistant strictly (required by most LLM templates).
//! The speaking agent is identified only through the content prefix [Speaker]:.
//!
//!   system  (World system prompt or Explorer system prompt — injected per call)
//!   user    "[Narrator]: Begin a new scene..."
//!   assistant "[World]: You stand at the entrance of a dark cave..."
//!   user    "[Explorer]: I light my torch and step inside."
//!   assistant "[World]: The torchlight reveals glittering crystals..."
//!   user    "[Explorer]: I examine the largest crystal."
//!   ...
//!
//! World always speaks as `assistant`, Explorer always speaks as `user`.
//! This maps cleanly onto the alternating constraint and matches the
//! conversational intent (World responds to Explorer actions).

use agent_runtime::llm::types::{ChatMessage, Role};
use agent_runtime::llm::LlamaClient;
use agent_runtime::types::{AgentInput, AgentInputMetadata};
use agent_runtime::{Agent, AgentConfig, EventStream};
use chrono::Local;
use std::sync::Arc;
use tokio::task;

const BASE_URL: &str = "https://192.168.91.57";
const MODEL: &str = "zai-org/glm-4.6v-flash";
const TURNS: usize = 3; // full World→Actor exchanges

// ── Scene runner ──────────────────────────────────────────────────────────────

struct SceneRunner {
    world: Agent,
    actor: Agent,
    /// Shared history visible to both agents.
    /// Every message is User-role with an attribution prefix.
    history: Vec<ChatMessage>,
    event_stream: EventStream,
}

impl SceneRunner {
    fn new(world: Agent, actor: Agent, event_stream: EventStream) -> Self {
        Self {
            world,
            actor,
            history: Vec::new(),
            event_stream,
        }
    }

    fn push(&mut self, speaker: &str, content: &str, role: Role) {
        let msg = ChatMessage {
            role,
            content: format!("[{}]: {}", speaker, content),
            tool_calls: None,
            tool_call_id: None,
            agent_id: Some(speaker.to_string()),
            workflow_id: Some("scene".to_string()),
        };
        self.history.push(msg);
    }

    /// Run TURNS full World → Actor exchanges.
    async fn run(&mut self, turns: usize) -> Result<(), String> {
        for turn in 1..=turns {
            println!("\n{}", "─".repeat(60));
            println!("  Turn {}/{}", turn, turns);
            println!("{}", "─".repeat(60));

            // ── World describes the scene / result of last action ──────────
            println!("\n[{}] 🌍 World:\n", ts());
            let world_response = {
                let input = AgentInput {
                    data: serde_json::Value::Null,
                    metadata: AgentInputMetadata {
                        step_index: turn,
                        previous_agent: Some("Actor".to_string()),
                    },
                    chat_history: Some(self.history.clone()),
                };
                self.world
                    .execute_with_events(input, Some(&self.event_stream))
                    .await
                    .map_err(|e| e.to_string())?
                    .data["response"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            // World is always `assistant` — it responds to Explorer user turns
            self.push("World", &world_response, Role::Assistant);

            // ── Actor decides what to do ───────────────────────────────────
            println!("\n[{}] 🧭 Explorer:\n", ts());
            let actor_response = {
                let input = AgentInput {
                    data: serde_json::Value::Null,
                    metadata: AgentInputMetadata {
                        step_index: turn,
                        previous_agent: Some("World".to_string()),
                    },
                    chat_history: Some(self.history.clone()),
                };
                self.actor
                    .execute_with_events(input, Some(&self.event_stream))
                    .await
                    .map_err(|e| e.to_string())?
                    .data["response"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            // Explorer is always `user` — it drives the conversation forward
            self.push("Explorer", &actor_response, Role::User);
        }

        Ok(())
    }
}

// ── Event monitor ─────────────────────────────────────────────────────────────

fn spawn_monitor(event_stream: &EventStream) -> tokio::task::JoinHandle<()> {
    let mut events = event_stream.subscribe();
    task::spawn(async move {
        use agent_runtime::event::{EventScope, EventType};
        while let Ok(event) = events.recv().await {
            match (&event.scope, &event.event_type) {
                (EventScope::LlmRequest, EventType::Progress) => {
                    if let Some(chunk) = event.data.get("chunk").and_then(|v| v.as_str()) {
                        print!("{}", chunk);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                    }
                }
                (EventScope::LlmRequest, EventType::Completed) => {
                    println!();
                }
                _ => {}
            }
        }
    })
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[{}] === Scene + Actor Demo ===", ts());
    println!("  Model  : {}", MODEL);
    println!("  Turns  : {}\n", TURNS);

    let client = Arc::new(LlamaClient::insecure(BASE_URL, MODEL));
    let event_stream = EventStream::new();
    let _monitor = spawn_monitor(&event_stream);

    let world_agent = Agent::new(
        AgentConfig::builder("World")
            .system_prompt(
                "You are the narrator and world engine of an interactive exploration story. \
                 You describe the environment vividly and react to the Explorer's actions. \
                 Be specific and sensory — describe what is seen, heard, felt. \
                 Keep each description to 3-4 sentences. \
                 The Explorer's actions shape what happens next.",
            )
            .strip_think_blocks(true)
            .build(),
    )
    .with_llm_client(client.clone());

    let actor_agent = Agent::new(
        AgentConfig::builder("Explorer")
            .system_prompt(
                "You are a bold and curious explorer navigating an unknown environment. \
                 Read the World's description carefully and decide on ONE specific action \
                 to take next. State your action clearly starting with 'I ...' \
                 Keep it to 1-2 sentences.",
            )
            .strip_think_blocks(true)
            .build(),
    )
    .with_llm_client(client.clone());

    let mut runner = SceneRunner::new(world_agent, actor_agent, event_stream);

    // Seed: give the World its opening scenario — as `user` so World responds as `assistant`
    runner.push(
        "Narrator",
        "Begin a new exploration scene. The Explorer has just arrived somewhere intriguing. \
         Set the opening scene.",
        Role::User,
    );

    if let Err(e) = runner.run(TURNS).await {
        eprintln!("\n[{}] ❌ Error: {}", ts(), e);
    }

    // ── Print full attributed transcript ──────────────────────────────────
    println!("\n\n{}", "═".repeat(60));
    println!("  Full transcript");
    println!("{}", "═".repeat(60));
    for msg in &runner.history {
        if msg.role == Role::System {
            continue;
        }
        let speaker = msg.agent_id.as_deref().unwrap_or("narrator");
        println!("\n[{}]\n{}", speaker, msg.content);
    }

    println!("\n[{}] Done.", ts());
    Ok(())
}

fn ts() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}
