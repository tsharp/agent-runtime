//! Turn-based multi-agent chat.
//!
//! The DM types the intent for each turn, then the active agents decide whether
//! to respond, pass, or leave. Once an agent leaves, they are removed from future turns.

use agent_runtime::llm::types::{ChatMessage, Role};
use agent_runtime::llm::LlamaClient;
use agent_runtime::types::AgentInput;
use agent_runtime::{Agent, AgentConfig, EventStream};
use chrono::Local;
use serde::Deserialize;
use std::io::{self, Write};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct Config {
    llm: LlmConfig,
    discussion: DiscussionConfig,
    #[serde(default = "default_agents_dir")]
    agents_dir: String,
    #[serde(skip)]
    agents: Vec<AgentDef>,
}

fn default_agents_dir() -> String {
    "./agents".into()
}

#[derive(Debug, Deserialize)]
struct LlmConfig {
    base_url: String,
    model: String,
}

#[derive(Debug, Deserialize)]
struct DiscussionConfig {
    topic: String,
}

#[derive(Debug, Deserialize, Clone)]
struct AgentDef {
    name: String,
    system_prompt: String,
}

impl Config {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::var("DISCOURSE_CONFIG")
            .unwrap_or_else(|_| format!("{}/discourse.yaml", env!("CARGO_MANIFEST_DIR")));

        let mut cfg: Config = serde_yaml::from_str(&std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read '{}': {}", path, e))?)
            .map_err(|e| format!("Failed to parse '{}': {}", path, e))?;

        let agents_dir = std::path::Path::new(&path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(&cfg.agents_dir);

        let mut paths: Vec<_> = std::fs::read_dir(&agents_dir)
            .map_err(|e| format!("Cannot read agents dir '{}': {}", agents_dir.display(), e))?
            .filter_map(|e| e.ok())
            .filter(|e| matches!(e.path().extension().and_then(|s| s.to_str()), Some("yaml" | "yml")))
            .map(|e| e.path())
            .collect();
        paths.sort();

        cfg.agents = paths
            .iter()
            .map(|p| {
                serde_yaml::from_str(&std::fs::read_to_string(p)
                    .map_err(|e| format!("Failed to read '{}': {}", p.display(), e))?)
                    .map_err(|e| format!("Failed to parse '{}': {}", p.display(), e).into())
            })
            .collect::<Result<Vec<AgentDef>, Box<dyn std::error::Error>>>()?;

        if cfg.agents.is_empty() {
            return Err(format!("No agent YAML files found in '{}'.", agents_dir.display()).into());
        }

        Ok(cfg)
    }
}

struct Actor {
    name: String,
    agent: Agent,
    left: bool,
}

enum Action {
    Respond(String),
    Pass,
    Leave,
}

fn parse_action(response: &str) -> Action {
    let response = response.trim();

    if response.starts_with("[LEAVE]") {
        Action::Leave
    } else if response.starts_with("[PASS]") || response.len() < 20 {
        Action::Pass
    } else {
        Action::Respond(response.strip_prefix("[RESPOND]").unwrap_or(response).trim().to_string())
    }
}

struct Moderator {
    topic: String,
    history: Vec<ChatMessage>,
    agents: Vec<Actor>,
    event_stream: EventStream,
}

impl Moderator {
    fn new(cfg: Config, event_stream: EventStream) -> Self {
        let client = Arc::new(LlamaClient::new(&cfg.llm.base_url, &cfg.llm.model));

        let agents = cfg
            .agents
            .into_iter()
            .map(|def| Actor {
                name: def.name.clone(),
                agent: Agent::new(
                    AgentConfig::builder(&def.name)
                        .system_prompt(&def.system_prompt)
                        .strip_think_blocks(false)
                        .build(),
                )
                .with_llm_client(client.clone()),
                left: false,
            })
            .collect();

        Self {
            topic: cfg.discussion.topic,
            history: vec![ChatMessage::user(
                "You are the Dungeon Master. Relay the intent of the players to the chat, then let the agents answer."
                    .to_string(),
            )],
            agents,
            event_stream,
        }
    }

    fn history_text(&self) -> String {
        self.history
            .iter()
            .map(|message| {
                let role = match message.role {
                    Role::System => "SYSTEM",
                    Role::User => "USER",
                    Role::Assistant => "ASSISTANT",
                    Role::Tool => "TOOL",
                };
                format!("[{}]\n{}", role, message.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn active_count(&self) -> usize {
        self.agents.iter().filter(|actor| !actor.left).count()
    }

    async fn run_turn(&mut self, round: usize, intent: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut order: Vec<usize> = self
            .agents
            .iter()
            .enumerate()
            .filter(|(_, actor)| !actor.left)
            .map(|(index, _)| index)
            .collect();
        order.sort_by_key(|_| rand::random::<u64>());

        println!("\n{}", "═".repeat(70));
        println!("  ROUND {}", round);
        println!("  DM intent: {}", intent.trim());
        println!("{}", "═".repeat(70));

        let mut spoke = 0;
        let mut left = 0;

        for index in order {
            let history_snapshot = self.history_text();
            let actor = &mut self.agents[index];

            let prompt = format!(
                "CURRENT SCENE:\n{}\n\nDM INTENT:\n{}\n\nYou are {}. Reply with exactly one of these forms:\n[RESPOND] your contribution\n[PASS]\n[LEAVE]\n\nLeave if you are done with the scene or have nothing more to add.",
                history_snapshot,
                intent.trim(),
                actor.name
            );

            println!("\n  🤔 {} is thinking...", actor.name);

            let output = actor
                .agent
                .execute_with_events(AgentInput::from_text(&prompt), Some(&self.event_stream))
                .await?;
            let raw = output
                .data
                .get("response")
                .and_then(|value| value.as_str())
                .unwrap_or("");

            match parse_action(raw) {
                Action::Pass => println!("     ✓ {} passes", actor.name),
                Action::Respond(content) => {
                    println!("\n  💬 {} speaks:", actor.name);
                    println!("     {}", content);
                    self.history
                        .push(ChatMessage::user(format!("[{}]: {}", actor.name, content)));
                    spoke += 1;
                }
                Action::Leave => {
                    println!("     ✦ {} leaves the scene", actor.name);
                    actor.left = true;
                    left += 1;
                    self.history
                        .push(ChatMessage::user(format!("[{} leaves the scene]", actor.name)));
                }
            }
        }

        self.agents.retain(|actor| !actor.left);
        println!("\n  ─ {} spoke, {} left, {} remain", spoke, left, self.active_count());
        Ok(())
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n{}", "█".repeat(70));
        println!("  DUNGEON CHAT");
        println!("  Topic: {}", self.topic);
        println!("  Active agents start: {}", self.active_count());
        println!("{}\n", "█".repeat(70));

        let mut round = 1;
        while !self.agents.is_empty() {
            print!("\nDM intent for round {} > ", round);
            io::stdout().flush()?;

            let mut intent = String::new();
            io::stdin().read_line(&mut intent)?;
            if intent.trim().is_empty() {
                println!("No intent entered, ending.");
                break;
            }

            self.run_turn(round, &intent).await?;
            round += 1;
        }

        self.print_summary();
        Ok(())
    }

    fn print_summary(&self) {
        println!("\n{}", "═".repeat(70));
        println!("  FINAL DISCUSSION SUMMARY");
        println!("{}", "═".repeat(70));

        for message in self.history.iter().filter(|message| message.role == Role::User) {
            println!("{}", message.content);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[{}] Loading config...", Local::now().format("%H:%M:%S"));

    let config = Config::load()?;

    println!(
        "[{}] {} agents | topic: {}",
        Local::now().format("%H:%M:%S"),
        config.agents.len(),
        config.discussion.topic
    );

    let event_stream = EventStream::new();
    let mut moderator = Moderator::new(config, event_stream);
    moderator.run().await?;

    println!("\n[{}] Done.", Local::now().format("%H:%M:%S"));
    Ok(())
}
