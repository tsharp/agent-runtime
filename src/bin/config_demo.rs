use agent_runtime::{RuntimeConfig, RetryPolicy, TimeoutConfig};

/// Example demonstrating configuration management
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Configuration Management Demo ===\n");

    // Example 1: Default configuration
    demo_default_config();

    // Example 2: Load from TOML file
    demo_toml_config()?;

    // Example 3: Environment variables
    demo_env_config();

    // Example 4: Programmatic configuration
    demo_programmatic_config();

    // Example 5: Configuration validation
    demo_validation()?;

    // Example 6: Convert to runtime types
    demo_conversion();

    println!("\n✅ All configuration examples completed!");

    Ok(())
}

fn demo_default_config() {
    println!("📋 Example 1: Default Configuration\n");

    let config = RuntimeConfig::default();

    println!("Retry Settings:");
    println!("  Max Attempts: {}", config.retry.max_attempts);
    println!("  Initial Delay: {}ms", config.retry.initial_delay_ms);
    println!("  Backoff Multiplier: {}", config.retry.backoff_multiplier);

    println!("\nTimeout Settings:");
    println!("  Total: {:?}ms", config.timeout.total_ms);
    println!("  First Response: {:?}ms", config.timeout.first_response_ms);

    println!("\nLogging Settings:");
    println!("  Level: {}", config.logging.level);
    println!("  Directory: {}", config.logging.directory);

    println!("\nWorkflow Settings:");
    println!("  Max Tool Iterations: {}", config.workflow.max_tool_iterations);

    println!();
}

fn demo_toml_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 Example 2: Load from Files\n");

    // Try to load from TOML file
    println!("Loading from TOML file...");
    match RuntimeConfig::from_toml_file("agent-runtime.toml") {
        Ok(config) => {
            println!("✅ Loaded from agent-runtime.toml");
            println!("   LLM Provider: {:?}", config.llm.default_provider);
            println!("   Default Model: {:?}", config.llm.default_model);
            println!("   Retry Attempts: {}", config.retry.max_attempts);
        }
        Err(e) => {
            println!("ℹ️  Could not load TOML file ({})", e);
        }
    }

    // Try to load from YAML file
    println!("\nLoading from YAML file...");
    match RuntimeConfig::from_yaml_file("agent-runtime.yaml") {
        Ok(config) => {
            println!("✅ Loaded from agent-runtime.yaml");
            println!("   LLM Provider: {:?}", config.llm.default_provider);
            println!("   Default Model: {:?}", config.llm.default_model);
            println!("   Retry Attempts: {}", config.retry.max_attempts);
        }
        Err(e) => {
            println!("ℹ️  Could not load YAML file ({})", e);
        }
    }

    // Auto-detect format from extension
    println!("\nAuto-detecting format from extension...");
    match RuntimeConfig::from_file("agent-runtime.yaml") {
        Ok(config) => {
            println!("✅ Auto-detected and loaded YAML config");
            println!("   Log level: {}", config.logging.level);
        }
        Err(e) => {
            println!("ℹ️  Could not auto-load ({})", e);
        }
    }

    println!();
    Ok(())
}

fn demo_env_config() {
    println!("🌍 Example 3: Environment Variables\n");

    println!("Configuration can be overridden via environment variables:");
    println!("  AGENT_RUNTIME__RETRY__MAX_ATTEMPTS=5");
    println!("  AGENT_RUNTIME__LOGGING__LEVEL=debug");
    println!("  AGENT_RUNTIME__LLM__DEFAULT_MODEL=gpt-4");

    println!("\nTo load from env:");
    println!("  let config = RuntimeConfig::from_env()?;");

    println!();
}

fn demo_programmatic_config() {
    println!("⚙️  Example 4: Programmatic Configuration\n");

    let mut config = RuntimeConfig::default();

    // Customize retry settings
    config.retry.max_attempts = 5;
    config.retry.initial_delay_ms = 200;

    // Customize logging
    config.logging.level = "debug".to_string();
    config.logging.json_format = true;

    // Customize workflow
    config.workflow.max_tool_iterations = 10;

    println!("✅ Created custom configuration programmatically");
    println!("Retry attempts: {}", config.retry.max_attempts);
    println!("Log level: {}", config.logging.level);
    println!("Max tool iterations: {}", config.workflow.max_tool_iterations);

    println!();
}

fn demo_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("✅ Example 5: Configuration Validation\n");

    // Valid configuration
    let valid_config = RuntimeConfig::default();
    match valid_config.validate() {
        Ok(_) => println!("✅ Default configuration is valid"),
        Err(e) => println!("❌ Validation error: {}", e),
    }

    // Invalid configuration (bad temperature)
    let mut invalid_config = RuntimeConfig::default();
    invalid_config.llm.default_temperature = 3.0; // Invalid: > 2.0

    match invalid_config.validate() {
        Ok(_) => println!("✅ Configuration is valid"),
        Err(e) => println!("❌ Validation caught invalid temperature: {}", e),
    }

    // Invalid configuration (bad jitter factor)
    let mut invalid_config2 = RuntimeConfig::default();
    invalid_config2.retry.jitter_factor = 1.5; // Invalid: > 1.0

    match invalid_config2.validate() {
        Ok(_) => println!("✅ Configuration is valid"),
        Err(e) => println!("❌ Validation caught invalid jitter: {}", e),
    }

    println!();
    Ok(())
}

fn demo_conversion() {
    println!("🔄 Example 6: Convert to Runtime Types\n");

    let config = RuntimeConfig::default();

    // Convert retry config to RetryPolicy
    let retry_policy: RetryPolicy = config.retry.to_policy();
    println!("✅ Converted RetryConfig → RetryPolicy");
    println!("   Max attempts: {}", retry_policy.max_attempts);
    println!("   Initial delay: {:?}", retry_policy.initial_delay);

    // Convert timeout config to TimeoutConfig
    let timeout_config: TimeoutConfig = config.timeout.to_config();
    println!("\n✅ Converted TimeoutConfigSettings → TimeoutConfig");
    println!("   Total timeout: {:?}", timeout_config.total);
    println!("   First response: {:?}", timeout_config.first_response);

    println!();
}
