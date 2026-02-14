use crate::error::{ConfigError, ConfigErrorCode};
use crate::retry::RetryPolicy;
use crate::timeout::TimeoutConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Main runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// LLM provider configurations
    #[serde(default)]
    pub llm: LlmConfig,

    /// Retry policy configuration
    #[serde(default)]
    pub retry: RetryConfig,

    /// Timeout configuration
    #[serde(default)]
    pub timeout: TimeoutConfigSettings,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Workflow configuration
    #[serde(default)]
    pub workflow: WorkflowConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            retry: RetryConfig::default(),
            timeout: TimeoutConfigSettings::default(),
            logging: LoggingConfig::default(),
            workflow: WorkflowConfig::default(),
        }
    }
}

impl RuntimeConfig {
    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError {
            code: ConfigErrorCode::FileNotFound,
            message: format!("Failed to read config file: {}", e),
            field: Some(path.display().to_string()),
        })?;

        toml::from_str(&content).map_err(|e| ConfigError {
            code: ConfigErrorCode::ParseError,
            message: format!("Failed to parse TOML: {}", e),
            field: None,
        })
    }

    /// Load configuration from a YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError {
            code: ConfigErrorCode::FileNotFound,
            message: format!("Failed to read config file: {}", e),
            field: Some(path.display().to_string()),
        })?;

        serde_yaml::from_str(&content).map_err(|e| ConfigError {
            code: ConfigErrorCode::ParseError,
            message: format!("Failed to parse YAML: {}", e),
            field: None,
        })
    }

    /// Load configuration from a file (auto-detects format from extension)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match extension {
            "toml" => Self::from_toml_file(path),
            "yaml" | "yml" => Self::from_yaml_file(path),
            _ => Err(ConfigError {
                code: ConfigErrorCode::ParseError,
                message: format!(
                    "Unsupported file extension '{}'. Use .toml, .yaml, or .yml",
                    extension
                ),
                field: Some(path.display().to_string()),
            }),
        }
    }

    /// Load configuration from environment variables
    /// Prefix: AGENT_RUNTIME_
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut settings = config::Config::builder();

        // Add environment variables with prefix
        settings = settings.add_source(
            config::Environment::with_prefix("AGENT_RUNTIME")
                .separator("__")
                .try_parsing(true),
        );

        settings
            .build()
            .and_then(|c| c.try_deserialize())
            .map_err(|e| ConfigError {
                code: ConfigErrorCode::ParseError,
                message: format!("Failed to parse environment config: {}", e),
                field: None,
            })
    }

    /// Load configuration from multiple sources (file, then env overrides)
    pub fn from_sources<P: AsRef<Path>>(file_path: Option<P>) -> Result<Self, ConfigError> {
        let mut settings = config::Config::builder();

        // Start with defaults
        settings = settings.add_source(config::Config::try_from(&Self::default()).unwrap());

        // Add file if provided
        if let Some(path) = file_path {
            let path_str = path.as_ref().display().to_string();
            settings = settings.add_source(config::File::with_name(&path_str).required(false));
        }

        // Add environment variables (highest priority)
        settings = settings.add_source(
            config::Environment::with_prefix("AGENT_RUNTIME")
                .separator("__")
                .try_parsing(true),
        );

        settings
            .build()
            .and_then(|c| c.try_deserialize())
            .map_err(|e| ConfigError {
                code: ConfigErrorCode::ParseError,
                message: format!("Failed to build config: {}", e),
                field: None,
            })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate LLM config
        self.llm.validate()?;

        // Validate retry config
        self.retry.validate()?;

        // Validate timeout config
        self.timeout.validate()?;

        Ok(())
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider to use
    pub default_provider: Option<String>,

    /// OpenAI configuration
    pub openai: Option<OpenAIConfig>,

    /// Llama.cpp configuration
    pub llama: Option<LlamaConfig>,

    /// Default model name
    pub default_model: Option<String>,

    /// Default temperature
    #[serde(default = "default_temperature")]
    pub default_temperature: f32,

    /// Default max tokens
    pub default_max_tokens: Option<u32>,
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_provider: None,
            openai: None,
            llama: None,
            default_model: None,
            default_temperature: 0.7,
            default_max_tokens: None,
        }
    }
}

impl LlmConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if let Some(temp) = Some(self.default_temperature) {
            if temp < 0.0 || temp > 2.0 {
                return Err(ConfigError {
                    code: ConfigErrorCode::InvalidValue,
                    message: "Temperature must be between 0.0 and 2.0".to_string(),
                    field: Some("llm.default_temperature".to_string()),
                });
            }
        }
        Ok(())
    }
}

/// OpenAI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
}

/// Llama.cpp-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaConfig {
    pub base_url: String,
    pub insecure: bool,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial delay in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Jitter factor (0.0 - 1.0)
    #[serde(default = "default_jitter_factor")]
    pub jitter_factor: f64,
}

fn default_max_attempts() -> u32 {
    3
}
fn default_initial_delay_ms() -> u64 {
    100
}
fn default_max_delay_ms() -> u64 {
    30000
}
fn default_backoff_multiplier() -> f64 {
    2.0
}
fn default_jitter_factor() -> f64 {
    0.1
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.backoff_multiplier < 1.0 {
            return Err(ConfigError {
                code: ConfigErrorCode::InvalidValue,
                message: "Backoff multiplier must be >= 1.0".to_string(),
                field: Some("retry.backoff_multiplier".to_string()),
            });
        }

        if self.jitter_factor < 0.0 || self.jitter_factor > 1.0 {
            return Err(ConfigError {
                code: ConfigErrorCode::InvalidValue,
                message: "Jitter factor must be between 0.0 and 1.0".to_string(),
                field: Some("retry.jitter_factor".to_string()),
            });
        }

        Ok(())
    }

    /// Convert to RetryPolicy
    pub fn to_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_attempts: self.max_attempts,
            initial_delay: Duration::from_millis(self.initial_delay_ms),
            max_delay: Duration::from_millis(self.max_delay_ms),
            backoff_multiplier: self.backoff_multiplier,
            jitter_factor: self.jitter_factor,
            max_total_duration: None, // Can be added if needed
        }
    }
}

/// Timeout configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfigSettings {
    /// Total timeout in milliseconds
    pub total_ms: Option<u64>,

    /// First response timeout in milliseconds
    pub first_response_ms: Option<u64>,
}

impl Default for TimeoutConfigSettings {
    fn default() -> Self {
        Self {
            total_ms: Some(300000),         // 5 minutes
            first_response_ms: Some(30000), // 30 seconds
        }
    }
}

impl TimeoutConfigSettings {
    fn validate(&self) -> Result<(), ConfigError> {
        // Validation passes for now
        Ok(())
    }

    /// Convert to TimeoutConfig
    pub fn to_config(&self) -> TimeoutConfig {
        TimeoutConfig {
            total: self.total_ms.map(Duration::from_millis),
            first_response: self.first_response_ms.map(Duration::from_millis),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log output directory
    #[serde(default = "default_log_dir")]
    pub directory: String,

    /// Enable JSON format
    #[serde(default)]
    pub json_format: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_dir() -> String {
    "output".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            directory: "output".to_string(),
            json_format: false,
        }
    }
}

/// Workflow execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Maximum concurrent workflows
    pub max_concurrent: Option<usize>,

    /// Maximum tool iterations per agent
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: u32,
}

fn default_max_tool_iterations() -> u32 {
    5
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_concurrent: None,
            max_tool_iterations: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.retry.max_attempts, 3);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.workflow.max_tool_iterations, 5);
    }

    #[test]
    fn test_toml_serialization() {
        let config = RuntimeConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("max_attempts"));
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
            [retry]
            max_attempts = 5
            initial_delay_ms = 200

            [logging]
            level = "debug"
            directory = "logs"
        "#;

        let config: RuntimeConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.retry.max_attempts, 5);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_yaml_serialization() {
        let config = RuntimeConfig::default();
        let yaml_str = serde_yaml::to_string(&config).unwrap();
        assert!(yaml_str.contains("max_attempts"));
    }

    #[test]
    fn test_yaml_deserialization() {
        let yaml_str = r#"
retry:
  max_attempts: 5
  initial_delay_ms: 200

logging:
  level: debug
  directory: logs
        "#;

        let config: RuntimeConfig = serde_yaml::from_str(yaml_str).unwrap();
        assert_eq!(config.retry.max_attempts, 5);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_retry_config_to_policy() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 200,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.2,
        };

        let policy = config.to_policy();
        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert_eq!(policy.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_validation_invalid_temperature() {
        let mut config = LlmConfig::default();
        config.default_temperature = 3.0; // Invalid

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_jitter() {
        let mut config = RetryConfig::default();
        config.jitter_factor = 1.5; // Invalid

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_timeout_config_conversion() {
        let settings = TimeoutConfigSettings {
            total_ms: Some(5000),
            first_response_ms: Some(1000),
        };

        let timeout = settings.to_config();
        assert_eq!(timeout.total, Some(Duration::from_millis(5000)));
        assert_eq!(timeout.first_response, Some(Duration::from_millis(1000)));
    }
}
