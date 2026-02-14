use crate::error::RuntimeError;
use std::time::Duration;
use tokio::time::timeout;

/// Configuration for operation timeouts
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Overall timeout for the entire operation
    pub total: Option<Duration>,

    /// Timeout for first response (useful for streaming)
    pub first_response: Option<Duration>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            total: Some(Duration::from_secs(300)), // 5 minutes default
            first_response: Some(Duration::from_secs(30)), // 30 seconds for first response
        }
    }
}

impl TimeoutConfig {
    /// No timeout (operations can run indefinitely)
    pub fn none() -> Self {
        Self {
            total: None,
            first_response: None,
        }
    }

    /// Quick timeout for fast operations
    pub fn quick() -> Self {
        Self {
            total: Some(Duration::from_secs(30)),
            first_response: Some(Duration::from_secs(5)),
        }
    }

    /// Long timeout for expensive operations
    pub fn long() -> Self {
        Self {
            total: Some(Duration::from_secs(600)), // 10 minutes
            first_response: Some(Duration::from_secs(60)),
        }
    }

    /// Custom timeout
    pub fn custom(total: Duration, first_response: Option<Duration>) -> Self {
        Self {
            total: Some(total),
            first_response,
        }
    }

    /// Execute an async operation with timeout protection
    ///
    /// # Example
    /// ```no_run
    /// use agent_runtime::timeout::TimeoutConfig;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<String, agent_runtime::RuntimeError> {
    /// let config = TimeoutConfig::default();
    /// let result = config.execute(
    ///     "fetch_data",
    ///     async {
    ///         // Your operation here
    ///         Ok("success".to_string())
    ///     }
    /// ).await?;
    /// # Ok(result)
    /// # }
    /// ```
    pub async fn execute<F, T>(&self, operation_name: &str, operation: F) -> Result<T, RuntimeError>
    where
        F: std::future::Future<Output = Result<T, RuntimeError>>,
    {
        if let Some(timeout_duration) = self.total {
            let start = std::time::Instant::now();

            match timeout(timeout_duration, operation).await {
                Ok(result) => result,
                Err(_) => Err(RuntimeError::Timeout {
                    operation: operation_name.to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        } else {
            // No timeout configured
            operation.await
        }
    }

    /// Execute with first response timeout (useful for streaming)
    ///
    /// Returns a tuple of (first_chunk, remaining_stream)
    pub async fn execute_with_first_response<F, T>(
        &self,
        operation_name: &str,
        mut operation: F,
    ) -> Result<T, RuntimeError>
    where
        F: std::future::Future<Output = Result<T, RuntimeError>> + Unpin,
    {
        if let Some(first_timeout) = self.first_response {
            let start = std::time::Instant::now();

            // Wait for first response with timeout
            match timeout(first_timeout, &mut operation).await {
                Ok(result) => result,
                Err(_) => Err(RuntimeError::Timeout {
                    operation: format!("{} (first response)", operation_name),
                    duration_ms: start.elapsed().as_millis() as u64,
                }),
            }
        } else {
            operation.await
        }
    }
}

/// Execute an operation with a specific timeout duration
///
/// Convenience function for one-off timeouts
///
/// # Example
/// ```no_run
/// use agent_runtime::timeout::with_timeout;
/// use std::time::Duration;
///
/// # async fn example() -> Result<String, agent_runtime::RuntimeError> {
/// let result = with_timeout(
///     Duration::from_secs(30),
///     "api_call",
///     async {
///         // Your operation
///         Ok("done".to_string())
///     }
/// ).await?;
/// # Ok(result)
/// # }
/// ```
pub async fn with_timeout<F, T>(
    duration: Duration,
    operation_name: &str,
    operation: F,
) -> Result<T, RuntimeError>
where
    F: std::future::Future<Output = Result<T, RuntimeError>>,
{
    let config = TimeoutConfig::custom(duration, None);
    config.execute(operation_name, operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LlmError;

    #[tokio::test]
    async fn test_operation_completes_within_timeout() {
        let config = TimeoutConfig::default();

        let result: Result<&str, RuntimeError> = config
            .execute("test_op", async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok("success")
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_operation_exceeds_timeout() {
        let config = TimeoutConfig::custom(Duration::from_millis(50), None);

        let result: Result<&str, RuntimeError> = config
            .execute("test_op", async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok("success")
            })
            .await;

        assert!(result.is_err());

        match result.unwrap_err() {
            RuntimeError::Timeout {
                operation,
                duration_ms,
            } => {
                assert_eq!(operation, "test_op");
                assert!(duration_ms >= 50);
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[tokio::test]
    async fn test_no_timeout_allows_long_operations() {
        let config = TimeoutConfig::none();

        let result: Result<&str, RuntimeError> = config
            .execute("test_op", async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("success")
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_timeout_convenience_function() {
        let result: Result<&str, RuntimeError> =
            with_timeout(Duration::from_secs(1), "test_op", async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok("success")
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_timeout_with_error_result() {
        let config = TimeoutConfig::default();

        let result: Result<&str, RuntimeError> = config
            .execute("test_op", async {
                Err(LlmError::network("Network error").into())
            })
            .await;

        assert!(result.is_err());

        // Should get the actual error, not a timeout
        match result.unwrap_err() {
            RuntimeError::Llm(_) => {
                // Expected
            }
            _ => panic!("Expected LLM error, not timeout"),
        }
    }

    #[tokio::test]
    async fn test_quick_timeout_config() {
        let config = TimeoutConfig::quick();
        assert_eq!(config.total, Some(Duration::from_secs(30)));
        assert_eq!(config.first_response, Some(Duration::from_secs(5)));
    }

    #[tokio::test]
    async fn test_long_timeout_config() {
        let config = TimeoutConfig::long();
        assert_eq!(config.total, Some(Duration::from_secs(600)));
        assert_eq!(config.first_response, Some(Duration::from_secs(60)));
    }
}
