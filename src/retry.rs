use crate::error::RuntimeError;
use rand::Rng;
use std::time::Duration;

/// Policy for retrying failed operations with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_attempts: u32,
    
    /// Initial delay before first retry
    pub initial_delay: Duration,
    
    /// Maximum delay between retries
    pub max_delay: Duration,
    
    /// Multiplier for exponential backoff (typically 2.0)
    pub backoff_multiplier: f64,
    
    /// Add random jitter to prevent thundering herd (0.0 - 1.0)
    /// 0.0 = no jitter, 1.0 = full jitter (delay * random(0-1))
    pub jitter_factor: f64,
    
    /// Maximum total duration for all retries
    pub max_total_duration: Option<Duration>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_total_duration: Some(Duration::from_secs(60)),
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with custom settings
    pub fn new(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            ..Default::default()
        }
    }
    
    /// Disable retries
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            ..Default::default()
        }
    }
    
    /// Aggressive retry for critical operations
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 1.5,
            jitter_factor: 0.2,
            max_total_duration: Some(Duration::from_secs(30)),
        }
    }
    
    /// Conservative retry for expensive operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 2,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 3.0,
            jitter_factor: 0.1,
            max_total_duration: Some(Duration::from_secs(120)),
        }
    }
    
    /// Calculate delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);
        
        let clamped = base_delay.min(self.max_delay.as_millis() as f64);
        
        // Add jitter
        let jittered = if self.jitter_factor > 0.0 {
            let mut rng = rand::thread_rng();
            let jitter = rng.gen::<f64>() * self.jitter_factor * clamped;
            clamped + jitter
        } else {
            clamped
        };
        
        Duration::from_millis(jittered as u64)
    }
    
    /// Execute an async operation with retry logic
    ///
    /// # Example
    /// ```no_run
    /// use agent_runtime::retry::RetryPolicy;
    /// use agent_runtime::{RuntimeError, LlmError};
    ///
    /// # async fn example() -> Result<String, RuntimeError> {
    /// let policy = RetryPolicy::default();
    /// let result = policy.execute(
    ///     "fetch_data",
    ///     || async {
    ///         // Your operation here - returns Result<T, impl Into<RuntimeError>>
    ///         Ok::<String, LlmError>("success".to_string())
    ///     }
    /// ).await?;
    /// # Ok(result)
    /// # }
    /// ```
    pub async fn execute<F, Fut, T, E>(
        &self,
        operation_name: &str,
        mut operation: F,
    ) -> Result<T, RuntimeError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: Into<RuntimeError> + Clone,
    {
        let start = std::time::Instant::now();
        let mut last_error = None;
        
        for attempt in 0..=self.max_attempts {
            // Check if we've exceeded max total duration
            if let Some(max_duration) = self.max_total_duration {
                if start.elapsed() > max_duration {
                    break;
                }
            }
            
            // Execute the operation
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let runtime_error: RuntimeError = e.into();
                    
                    // Check if error is retryable
                    let should_retry = match &runtime_error {
                        RuntimeError::Llm(llm_err) => llm_err.is_retryable(),
                        _ => false, // Only retry LLM errors for now
                    };
                    
                    last_error = Some(runtime_error.clone());
                    
                    // Don't retry if:
                    // - This was the last attempt
                    // - Error is not retryable
                    if attempt >= self.max_attempts || !should_retry {
                        break;
                    }
                    
                    // Calculate delay and sleep
                    let delay = self.delay_for_attempt(attempt);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        
        // All attempts exhausted
        Err(RuntimeError::RetryExhausted {
            operation: operation_name.to_string(),
            attempts: self.max_attempts + 1,
            last_error: Box::new(last_error.unwrap()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LlmError;
    
    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // No jitter for predictable tests
            max_total_duration: None,
        };
        
        assert_eq!(policy.delay_for_attempt(0).as_millis(), 100);
        assert_eq!(policy.delay_for_attempt(1).as_millis(), 200);
        assert_eq!(policy.delay_for_attempt(2).as_millis(), 400);
    }
    
    #[test]
    fn test_max_delay_clamp() {
        let policy = RetryPolicy {
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
            max_total_duration: None,
        };
        
        // After enough attempts, should clamp to max_delay
        let delay = policy.delay_for_attempt(10);
        assert_eq!(delay, Duration::from_secs(5));
    }
    
    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let policy = RetryPolicy::default();
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        
        let result: Result<&str, RuntimeError> = policy
            .execute("test_op", move || {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if count == 1 {
                        Err(LlmError::network("Network error"))
                    } else {
                        Ok("success")
                    }
                }
            })
            .await;
        
        assert!(result.is_ok());
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
    
    #[tokio::test]
    async fn test_retry_exhausted() {
        let policy = RetryPolicy::new(2, Duration::from_millis(10));
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        
        let result: Result<&str, RuntimeError> = policy
            .execute("test_op", move || {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(LlmError::network("Network error"))
                }
            })
            .await;
        
        assert!(result.is_err());
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 3); // Initial + 2 retries
        
        match result.unwrap_err() {
            RuntimeError::RetryExhausted { attempts: retry_attempts, .. } => {
                assert_eq!(retry_attempts, 3);
            }
            _ => panic!("Expected RetryExhausted error"),
        }
    }
    
    #[tokio::test]
    async fn test_no_retry_on_non_retryable_error() {
        let policy = RetryPolicy::default();
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_clone = attempts.clone();
        
        let result: Result<&str, RuntimeError> = policy
            .execute("test_op", move || {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(LlmError {
                        code: crate::error::LlmErrorCode::InvalidRequest,
                        message: "Bad request".to_string(),
                        provider: None,
                        model: None,
                        retryable: false,
                    })
                }
            })
            .await;
        
        assert!(result.is_err());
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1); // Should not retry non-retryable errors
    }
}
