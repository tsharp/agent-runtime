//! Multi-turn multi-agent discourse demo.
//!
//! Two agents debate a topic across N rounds. Each agent's response is
//! stored in the shared history as a User-role message attributed to that
//! agent (e.g. "[Optimist]: ..."). This means every participant sees the
//! full conversation but the LLM never mistakes another agent's words for
//! its own prior assistant turn.
//!
//! Pattern:
//!   shared_history grows with attributed user messages
//!   ┌──────────┐        ┌──────────┐
//!   │ Optimist │◄──────►│ Skeptic  │
//!   └──────────┘        └──────────┘
//!        │                   │
//!        └────shared history──┘
//!              (user-attributed)

use agent_runtime::llm::types::{ChatMessage, Role};
use agent_runtime::llm::LlamaClient;
use agent_runtime::types::{AgentInput, AgentInputMetadata};
use agent_runtime::{Agent, AgentConfig, EventStream};
use chrono::Local;
use std::sync::Arc;
use tokio::task;

const BASE_URL: &str = "https://192.168.91.57";
const MODEL: &str = "zai-org/glm-4.6v-flash";

const TOPIC: &str = "AI will have a net positive impact on society";
const ROUNDS: usize = 2; // how many back-and-forth exchanges

// ── Orchestrator ──────────────────────────────────────────────────────────────

struct Participant {
    name: String,
    agent: Agent,
}

struct Orchestrator {
    participants: Vec<Participant>,
    /// Shared history — all messages visible to every participant.
    /// Agent responses are stored as User-role messages prefixed with
    /// the agent's name so no LLM confuses another agent's words for
    /// its own prior output.
    history: Vec<ChatMessage>,
    event_stream: EventStream,
}

impl Orchestrator {
    fn new(event_stream: EventStream) -> Self {
        Self {
            participants: Vec::new(),
            history: Vec::new(),
            event_stream,
        }
    }

    fn add_participant(&mut self, name: impl Into<String>, agent: Agent) {
        self.participants.push(Participant {
            name: name.into(),
            agent,
        });
    }

    /// Seed the conversation with a framing user message.
    fn seed(&mut self, message: impl Into<String>) {
        self.history.push(ChatMessage::user(message));
    }

    /// Push an attributed message, alternating user/assistant based on
    /// position in history (ignoring system messages). Attribution is
    /// embedded in the content as `[Name]: ...` so any agent can read it.
    fn push_attributed(&mut self, name: &str, content: &str) {
        // Determine role from the last non-system message
        let last_role = self.history.iter().rev()
            .find(|m| m.role != Role::System)
            .map(|m| &m.role);

        let role = match last_role {
            Some(Role::User) => Role::Assistant,
            _ => Role::User,
        };

        let msg = ChatMessage {
            role,
            content: format!("[{}]: {}", name, content),
            tool_calls: None,
            tool_call_id: None,
            agent_id: Some(name.to_string()),
            workflow_id: Some("discourse".to_string()),
        };
        self.history.push(msg);
    }
    async fn run_turn(&mut self, index: usize) -> Result<String, String> {
        let (name, input) = {
            let participant = &self.participants[index];
            let input = AgentInput {
                data: serde_json::Value::Null,
                metadata: AgentInputMetadata {
                    step_index: index,
                    previous_agent: None,
                },
                chat_history: Some(self.history.clone()),
            };
            (participant.name.clone(), input)
        };

        let output = self.participants[index]
            .agent
            .execute_with_events(input, Some(&self.event_stream))
            .await
            .map_err(|e| e.to_string())?;

        let response = output.data["response"]
            .as_str()
            .unwrap_or("")
            .to_string();

        self.push_attributed(&name, &response);

        Ok(response)
    }

    /// Run N full rounds — each round every participant takes one turn.
    async fn run_rounds(&mut self, rounds: usize) -> Result<(), String> {
        for round in 1..=rounds {
            println!("\n{}", "═".repeat(60));
            println!("  Round {}/{}", round, rounds);
            println!("{}", "═".repeat(60));

            for i in 0..self.participants.len() {
                let name = self.participants[i].name.clone();
                println!("\n[{}] 🎙️  {} speaking...\n", ts(), name);

                self.run_turn(i).await?;
            }
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
    println!("[{}] === Multi-Agent Discourse Demo ===", ts());
    println!("  Topic  : {}", TOPIC);
    println!("  Rounds : {}", ROUNDS);
    println!("  Model  : {}\n", MODEL);

    let client = Arc::new(LlamaClient::new(BASE_URL, MODEL));

    let event_stream = EventStream::new();
    let _monitor = spawn_monitor(&event_stream);

    let mut orchestrator = Orchestrator::new(event_stream);

    orchestrator.add_participant(
        "Optimist",
        Agent::new(
            AgentConfig::builder("Optimist")
                .system_prompt(
                    "You are an enthusiastic optimist. You are debating the topic: \
                     'AI will have a net positive impact on society'. \
                     Strongly advocate FOR this position. Keep your response to 3-4 sentences. \
                     Directly engage with the previous speaker's points when relevant.",
                )
                .strip_think_blocks(true)
                .build(),
        )
        .with_llm_client(client.clone()),
    );

    orchestrator.add_participant(
        "Skeptic",
        Agent::new(
            AgentConfig::builder("Skeptic")
                .system_prompt(
                    "You are a careful skeptic. You are debating the topic: \
                     'AI will have a net positive impact on society'. \
                     Challenge this position with concrete concerns. Keep your response to 3-4 sentences. \
                     Directly engage with the previous speaker's points.",
                )
                .strip_think_blocks(true)
                .build(),
        )
        .with_llm_client(client.clone()),
    );

    // Seed the conversation
    orchestrator.seed(format!(
        "Begin a structured debate on the following topic: \"{}\". \
         Each participant should argue their assigned position.",
        TOPIC
    ));

    // Run the discourse
    if let Err(e) = orchestrator.run_rounds(ROUNDS).await {
        eprintln!("\n[{}] ❌ Error: {}", ts(), e);
    }

    // ── Print full attributed history ─────────────────────────────────────
    println!("\n\n{}", "═".repeat(60));
    println!("  Full conversation history");
    println!("{}", "═".repeat(60));

    for msg in &orchestrator.history {
        if msg.role == Role::System {
            continue; // skip injected system prompts
        }
        let speaker = msg.agent_id.as_deref().unwrap_or(match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            _ => "system",
        });
        println!("\n[{}]\n{}", speaker, msg.content);
    }

    println!("\n[{}] Done.", ts());
    Ok(())
}

fn ts() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}
