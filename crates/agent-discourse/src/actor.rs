//! Actor definition: deserializes from YAML actor files and generates system prompts.

use serde::Deserialize;
use std::collections::HashMap;

/// Top-level wrapper matching the `version: 1 / actor: …` envelope.
#[derive(Debug, Deserialize)]
pub struct ActorFile {
    pub version: u32,
    pub actor: Actor,
}

impl ActorFile {
    pub fn from_yaml(src: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(src)
    }
}

/// A fully-described actor loaded from a YAML definition.
#[derive(Debug, Deserialize)]
pub struct Actor {
    pub id: String,
    #[serde(rename = "type")]
    pub actor_type: String,

    pub identity: Identity,
    pub system_context: SystemContext,
    pub voice: Voice,
    pub goals: Goals,
    pub behavior: Behavior,
    pub activation: Activation,
    pub knowledge: Knowledge,
    pub memory: Memory,
    pub capabilities: Capabilities,
}

impl Actor {
    /// Build a natural-language system prompt that an LLM can use as its persona.
    pub fn to_system_prompt(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Identity
        parts.push(format!(
            "You are {}, a {} ({}) in a {} world.",
            self.identity.name,
            self.identity.role,
            self.identity.archetype,
            self.system_context.world
        ));

        // Scene & mood
        parts.push(format!(
            "Current scene: {}. Mood: {}.",
            self.system_context.current_scene, self.system_context.mood
        ));

        if !self.system_context.known_threats.is_empty() {
            parts.push(format!(
                "Known threats: {}.",
                self.system_context.known_threats.join(", ")
            ));
        }

        if !self.system_context.party_members.is_empty() {
            parts.push(format!(
                "Party members present: {}.",
                self.system_context.party_members.join(", ")
            ));
        }

        // Voice
        parts.push(format!(
            "Your voice is {} and {}.",
            self.voice.tone, self.voice.style
        ));
        if !self.voice.speech_patterns.is_empty() {
            parts.push(format!(
                "Speech patterns: {}.",
                self.voice.speech_patterns.join("; ")
            ));
        }

        // Goals
        if !self.goals.primary.is_empty() {
            parts.push(format!(
                "Primary goals: {}.",
                self.goals.primary.join(", ")
            ));
        }
        if !self.goals.secondary.is_empty() {
            parts.push(format!(
                "Secondary goals: {}.",
                self.goals.secondary.join(", ")
            ));
        }

        // Behavior
        parts.push(format!(
            "Behavioral traits: {} initiative, {} risk tolerance, {} cooperation, {} emotional control.",
            self.behavior.initiative,
            self.behavior.risk_tolerance,
            self.behavior.cooperation,
            self.behavior.emotional_control,
        ));

        // Activation
        if !self.activation.speak_when.is_empty() {
            parts.push(format!(
                "Speak when: {}.",
                self.activation.speak_when.join(", ")
            ));
        }
        if !self.activation.remain_silent_when.is_empty() {
            parts.push(format!(
                "Remain silent when: {}.",
                self.activation.remain_silent_when.join(", ")
            ));
        }

        // Knowledge
        if !self.knowledge.public.is_empty() {
            parts.push(format!(
                "Public knowledge: {}.",
                self.knowledge.public.join("; ")
            ));
        }
        if !self.knowledge.private.is_empty() {
            parts.push(format!(
                "Private knowledge (known only to you): {}.",
                self.knowledge.private.join("; ")
            ));
        }

        // Memory
        if self.memory.persistent && !self.memory.remembers.is_empty() {
            parts.push(format!(
                "You remember: {}.",
                self.memory.remembers.join(", ")
            ));
        }

        // Capabilities
        if !self.capabilities.skills.is_empty() {
            parts.push(format!(
                "Skills: {}.",
                self.capabilities.skills.join(", ")
            ));
        }

        parts.join("\n")
    }
}

#[derive(Debug, Deserialize)]
pub struct Identity {
    pub name: String,
    pub role: String,
    pub archetype: String,
}

#[derive(Debug, Deserialize)]
pub struct SystemContext {
    pub world: String,
    pub current_scene: String,
    pub mood: String,
    #[serde(default)]
    pub known_threats: Vec<String>,
    #[serde(default)]
    pub party_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Voice {
    pub tone: String,
    pub style: String,
    #[serde(default)]
    pub speech_patterns: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Goals {
    #[serde(default)]
    pub primary: Vec<String>,
    #[serde(default)]
    pub secondary: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Behavior {
    pub initiative: String,
    pub risk_tolerance: String,
    pub cooperation: String,
    pub emotional_control: String,
}

#[derive(Debug, Deserialize)]
pub struct Activation {
    #[serde(default)]
    pub speak_when: Vec<String>,
    #[serde(default)]
    pub remain_silent_when: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Knowledge {
    #[serde(default)]
    pub public: Vec<String>,
    #[serde(default)]
    pub private: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Memory {
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub remembers: Vec<String>,
    /// emotion → subject → intensity (0.0–1.0)
    #[serde(default)]
    pub emotional_memory: HashMap<String, HashMap<String, f32>>,
}

#[derive(Debug, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub skills: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // Inline the fixture so tests don't depend on file-system paths.
    const MIRA_YAML: &str = r#"
version: 1

actor:
  id: mira
  type: npc

  identity:
    name: Mira Voss
    role: scout
    archetype: cautious survivor

  system_context:
    world: dark fantasy
    current_scene: ruined chapel
    mood: tense
    known_threats:
      - hidden undead
    party_members:
      - thorne
      - player

  voice:
    tone: wary
    style: concise
    speech_patterns:
      - observational
      - avoids long explanations
      - speaks concretely under stress

  goals:
    primary:
      - keep the party alive
      - avoid ambushes
    secondary:
      - gain the player's trust

  behavior:
    initiative: medium
    risk_tolerance: low
    cooperation: high
    emotional_control: steady

  activation:
    speak_when:
      - danger_detected
      - player_hesitates
      - new_information_appears

    remain_silent_when:
      - another_character_has_authority
      - situation_is_stable

  knowledge:
    public:
      - the chapel appears abandoned

    private:
      - the dust near the altar was recently disturbed

  memory:
    persistent: true

    remembers:
      - betrayals
      - promises
      - injuries
      - player_choices

    emotional_memory:
      trust:
        player: 0.4
      fear:
        undead: 0.8

  capabilities:
    skills:
      - stealth
      - tracking
      - perception
"#;

    fn mira() -> Actor {
        ActorFile::from_yaml(MIRA_YAML).unwrap().actor
    }

    // ── Deserialization ────────────────────────────────────────────────────

    #[test]
    fn parses_top_level_fields() {
        let actor = mira();
        assert_eq!(actor.id, "mira");
        assert_eq!(actor.actor_type, "npc");
    }

    #[test]
    fn parses_identity() {
        let actor = mira();
        assert_eq!(actor.identity.name, "Mira Voss");
        assert_eq!(actor.identity.role, "scout");
        assert_eq!(actor.identity.archetype, "cautious survivor");
    }

    #[test]
    fn parses_system_context() {
        let ctx = mira().system_context;
        assert_eq!(ctx.world, "dark fantasy");
        assert_eq!(ctx.current_scene, "ruined chapel");
        assert_eq!(ctx.mood, "tense");
        assert_eq!(ctx.known_threats, vec!["hidden undead"]);
        assert_eq!(ctx.party_members, vec!["thorne", "player"]);
    }

    #[test]
    fn parses_voice() {
        let voice = mira().voice;
        assert_eq!(voice.tone, "wary");
        assert_eq!(voice.style, "concise");
        assert_eq!(voice.speech_patterns.len(), 3);
        assert!(voice.speech_patterns.contains(&"observational".to_string()));
    }

    #[test]
    fn parses_goals() {
        let goals = mira().goals;
        assert_eq!(goals.primary, vec!["keep the party alive", "avoid ambushes"]);
        assert_eq!(goals.secondary, vec!["gain the player's trust"]);
    }

    #[test]
    fn parses_behavior() {
        let b = mira().behavior;
        assert_eq!(b.initiative, "medium");
        assert_eq!(b.risk_tolerance, "low");
        assert_eq!(b.cooperation, "high");
        assert_eq!(b.emotional_control, "steady");
    }

    #[test]
    fn parses_activation() {
        let act = mira().activation;
        assert!(act.speak_when.contains(&"danger_detected".to_string()));
        assert!(act.remain_silent_when.contains(&"situation_is_stable".to_string()));
    }

    #[test]
    fn parses_knowledge() {
        let k = mira().knowledge;
        assert_eq!(k.public, vec!["the chapel appears abandoned"]);
        assert_eq!(k.private, vec!["the dust near the altar was recently disturbed"]);
    }

    #[test]
    fn parses_memory() {
        let m = mira().memory;
        assert!(m.persistent);
        assert!(m.remembers.contains(&"betrayals".to_string()));
        assert!(m.remembers.contains(&"player_choices".to_string()));

        let trust = m.emotional_memory.get("trust").unwrap();
        let player_trust = trust.get("player").copied().unwrap();
        assert!((player_trust - 0.4).abs() < f32::EPSILON);

        let fear = m.emotional_memory.get("fear").unwrap();
        let undead_fear = fear.get("undead").copied().unwrap();
        assert!((undead_fear - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_capabilities() {
        let skills = mira().capabilities.skills;
        assert_eq!(skills, vec!["stealth", "tracking", "perception"]);
    }

    // ── System prompt generation ───────────────────────────────────────────

    #[test]
    fn system_prompt_contains_name_and_world() {
        let prompt = mira().to_system_prompt();
        assert!(prompt.contains("Mira Voss"), "prompt missing name:\n{prompt}");
        assert!(prompt.contains("dark fantasy"), "prompt missing world:\n{prompt}");
    }

    #[test]
    fn system_prompt_contains_scene_and_mood() {
        let prompt = mira().to_system_prompt();
        assert!(prompt.contains("ruined chapel"), "prompt missing scene:\n{prompt}");
        assert!(prompt.contains("tense"), "prompt missing mood:\n{prompt}");
    }

    #[test]
    fn system_prompt_contains_voice() {
        let prompt = mira().to_system_prompt();
        assert!(prompt.contains("wary"), "prompt missing tone:\n{prompt}");
        assert!(prompt.contains("concise"), "prompt missing style:\n{prompt}");
    }

    #[test]
    fn system_prompt_contains_primary_goals() {
        let prompt = mira().to_system_prompt();
        assert!(prompt.contains("keep the party alive"), "prompt missing primary goal:\n{prompt}");
        assert!(prompt.contains("avoid ambushes"), "prompt missing primary goal:\n{prompt}");
    }

    #[test]
    fn system_prompt_contains_private_knowledge() {
        let prompt = mira().to_system_prompt();
        assert!(
            prompt.contains("dust near the altar was recently disturbed"),
            "prompt missing private knowledge:\n{prompt}"
        );
    }

    #[test]
    fn system_prompt_contains_skills() {
        let prompt = mira().to_system_prompt();
        assert!(prompt.contains("stealth"), "prompt missing skills:\n{prompt}");
        assert!(prompt.contains("tracking"), "prompt missing skills:\n{prompt}");
    }

    #[test]
    fn system_prompt_contains_activation_triggers() {
        let prompt = mira().to_system_prompt();
        assert!(prompt.contains("danger_detected"), "prompt missing speak_when:\n{prompt}");
        assert!(
            prompt.contains("situation_is_stable"),
            "prompt missing remain_silent_when:\n{prompt}"
        );
    }

    // ── Snapshot: full prompt shape ────────────────────────────────────────

    #[test]
    fn system_prompt_snapshot() {
        let prompt = mira().to_system_prompt();
        // Print so `cargo test -- --nocapture` shows the generated context.
        println!("\n── Mira system prompt ──────────────────────────\n{prompt}\n────────────────────────────────────────────────");
        assert!(!prompt.is_empty());
    }

    // ── Minimal actor (only required fields with defaults) ─────────────────

    #[test]
    fn minimal_actor_deserializes() {
        let yaml = r#"
version: 1
actor:
  id: ghost
  type: enemy
  identity:
    name: The Hollow
    role: wraith
    archetype: silent predator
  system_context:
    world: gothic horror
    current_scene: fog-covered bridge
    mood: eerie
  voice:
    tone: silent
    style: non-verbal
  goals: {}
  behavior:
    initiative: low
    risk_tolerance: high
    cooperation: none
    emotional_control: absent
  activation: {}
  knowledge: {}
  memory:
    persistent: false
  capabilities: {}
"#;
        let actor = ActorFile::from_yaml(yaml).unwrap().actor;
        assert_eq!(actor.id, "ghost");
        assert!(actor.capabilities.skills.is_empty());
        assert!(actor.goals.primary.is_empty());
        let prompt = actor.to_system_prompt();
        assert!(prompt.contains("The Hollow"));
        assert!(prompt.contains("gothic horror"));
    }
}
