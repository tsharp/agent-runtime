use agent_runtime::{
    LlmError, RetryPolicy, RuntimeError, TimeoutConfig,
};
use std::time::Duration;

/// Example demonstrating production-ready error handling, retries, and timeouts
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Production Reliability Features Demo ===\n");

    // Example 1: Error Types
    demo_error_types();

    // Example 2: Retry with Exponential Backoff
    demo_retry_logic().await?;

    // Example 3: Timeout Protection
    demo_timeouts().await?;

    // Example 4: Combined - Retry + Timeout
    demo_combined().await?;

    println!("\n✅ All examples completed successfully!");

    Ok(())
}

fn demo_error_types() {
    println!("📋 Example 1: Comprehensive Error Types\n");

    // Network error (retryable)
    let network_err = LlmError::network("Connection refused");
    println!("Network Error: {}", network_err);
    println!("  Retryable: {}", network_err.is_retryable());

    // Rate limit error (retryable)
    let rate_limit_err = LlmError::rate_limit("Too many requests");
    println!("\nRate Limit Error: {}", rate_limit_err);
    println!("  Retryable: {}", rate_limit_err.is_retryable());

    // Invalid request error (not retryable)
    let invalid_err = LlmError {
        code: agent_runtime::LlmErrorCode::InvalidRequest,
        message: "Missing required field 'model'".to_string(),
        provider: Some("openai".to_string()),
        model: None,
        retryable: false,
    };
    println!("\nInvalid Request Error: {}", invalid_err);
    println!("  Retryable: {}", invalid_err.is_retryable());

    println!();
}

async fn demo_retry_logic() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Example 2: Retry with Exponential Backoff\n");

    // Simulate an operation that fails twice then succeeds
    let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    let policy = RetryPolicy::default();
    println!("Retry Policy:");
    println!("  Max Attempts: {}", policy.max_attempts);
    println!("  Initial Delay: {:?}", policy.initial_delay);
    println!("  Backoff Multiplier: {}", policy.backoff_multiplier);
    println!("  Jitter Factor: {}", policy.jitter_factor);
    println!();

    println!("Simulating flaky network operation...");
    let result = {
        let attempt_count = attempt_count.clone();
        policy
            .execute("api_call", move || {
                let attempt_count = attempt_count.clone();
                async move {
                    let count =
                        attempt_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    println!("  Attempt {}", count);

                    if count < 3 {
                        // Fail first 2 attempts
                        Err(LlmError::network("Connection timeout"))
                    } else {
                        // Succeed on 3rd attempt
                        Ok("Success!".to_string())
                    }
                }
            })
            .await
    };

    match result {
        Ok(data) => println!("✅ Operation succeeded: {}", data),
        Err(e) => println!("❌ Operation failed: {}", e),
    }

    println!(
        "Total attempts: {}\n",
        attempt_count.load(std::sync::atomic::Ordering::SeqCst)
    );

    Ok(())
}

async fn demo_timeouts() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️  Example 3: Timeout Protection\n");

    // Fast operation (completes within timeout)
    let config = TimeoutConfig::quick();
    println!("Quick Timeout Config:");
    println!("  Total: {:?}", config.total);
    println!("  First Response: {:?}", config.first_response);
    println!();

    println!("Running fast operation...");
    let result: Result<&str, RuntimeError> = config
        .execute("fast_op", async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok("Completed quickly")
        })
        .await;

    match result {
        Ok(data) => println!("✅ Fast operation: {}", data),
        Err(e) => println!("❌ Error: {}", e),
    }

    // Slow operation (exceeds timeout)
    println!("\nRunning slow operation (will timeout)...");
    let slow_config = TimeoutConfig::custom(Duration::from_millis(100), None);
    let result: Result<&str, RuntimeError> = slow_config
        .execute("slow_op", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok("This won't complete")
        })
        .await;

    match result {
        Ok(_) => println!("✅ Slow operation completed"),
        Err(RuntimeError::Timeout {
            operation,
            duration_ms,
        }) => {
            println!(
                "⏰ Operation '{}' timed out after {}ms (expected)",
                operation, duration_ms
            );
        }
        Err(e) => println!("❌ Unexpected error: {}", e),
    }

    println!();

    Ok(())
}

async fn demo_combined() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Example 4: Combined Retry + Timeout\n");

    // Use retry policy with timeout on each attempt
    let retry_policy = RetryPolicy::new(3, Duration::from_millis(50));
    let timeout_config = TimeoutConfig::custom(Duration::from_millis(200), None);

    println!("Configuration:");
    println!("  Max Retries: {}", retry_policy.max_attempts);
    println!("  Timeout per attempt: {:?}", timeout_config.total);
    println!();

    let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    println!("Executing operation with retry + timeout...");
    let result = {
        let attempt_count = attempt_count.clone();
        let timeout_config = timeout_config.clone();

        retry_policy
            .execute("combined_op", move || {
                let attempt_count = attempt_count.clone();
                let timeout_config = timeout_config.clone();

                async move {
                    let count =
                        attempt_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    println!("  Attempt {}", count);

                    // Wrap the operation in a timeout
                    timeout_config
                        .execute("operation", async {
                            // Simulate slow operation that gets faster
                            let delay_ms = 300 / count; // Gets faster each attempt
                            tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;

                            if count < 2 {
                                Err(LlmError::network("Still slow").into())
                            } else {
                                Ok("Success!")
                            }
                        })
                        .await
                }
            })
            .await
    };

    match result {
        Ok(data) => {
            println!("✅ Operation succeeded: {}", data);
            println!(
                "   Completed after {} attempts",
                attempt_count.load(std::sync::atomic::Ordering::SeqCst)
            );
        }
        Err(e) => println!("❌ Operation failed: {}", e),
    }

    println!();

    Ok(())
}
