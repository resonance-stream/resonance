//! Test helper functions for API integration tests
//!
//! Provides utility functions for setting up test environments,
//! creating mock servers, and asserting responses.

#![allow(dead_code)]

use std::collections::HashMap;

/// Temporarily set environment variables for the duration of a test
///
/// Returns a guard that will restore the original values when dropped.
pub struct EnvGuard {
    original: HashMap<String, Option<String>>,
}

impl EnvGuard {
    /// Create a new environment guard that sets the given variables
    pub fn new(vars: &[(String, String)]) -> Self {
        let mut original = HashMap::new();

        for (key, value) in vars {
            // Save the original value (or None if not set)
            original.insert(key.clone(), std::env::var(key).ok());
            // Set the new value
            std::env::set_var(key, value);
        }

        Self { original }
    }

    /// Create from a HashMap
    pub fn from_map(vars: &HashMap<String, String>) -> Self {
        let pairs: Vec<_> = vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        Self::new(&pairs)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.original {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Assert that a result contains a specific error message substring
#[macro_export]
macro_rules! assert_err_contains {
    ($result:expr, $substr:expr) => {
        match &$result {
            Ok(_) => panic!("Expected error but got Ok"),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains($substr),
                    "Error message '{}' does not contain '{}'",
                    msg,
                    $substr
                );
            }
        }
    };
}

/// Create a test database URL with a unique database name
pub fn test_database_url(test_name: &str) -> String {
    let sanitized = test_name.replace("::", "_").replace(" ", "_");
    format!(
        "postgres://resonance:resonance@localhost:5432/resonance_test_{}",
        sanitized
    )
}

/// Generate a random port number for testing servers
pub fn random_test_port() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static PORT: AtomicU16 = AtomicU16::new(9000);
    PORT.fetch_add(1, Ordering::SeqCst)
}

/// Wait for a condition with timeout
pub async fn wait_for<F, Fut>(condition: F, timeout_ms: u64, poll_interval_ms: u64) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let interval = std::time::Duration::from_millis(poll_interval_ms);

    loop {
        if condition().await {
            return true;
        }

        if start.elapsed() >= timeout {
            return false;
        }

        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_guard_sets_and_restores() {
        let key = "TEST_ENV_GUARD_VAR";
        std::env::remove_var(key);

        {
            let _guard = EnvGuard::new(&[(key.to_string(), "test_value".to_string())]);
            assert_eq!(std::env::var(key).unwrap(), "test_value");
        }

        // After guard is dropped, var should be removed
        assert!(std::env::var(key).is_err());
    }

    #[test]
    fn test_env_guard_restores_original() {
        let key = "TEST_ENV_GUARD_RESTORE_VAR";
        std::env::set_var(key, "original");

        {
            let _guard = EnvGuard::new(&[(key.to_string(), "modified".to_string())]);
            assert_eq!(std::env::var(key).unwrap(), "modified");
        }

        // After guard is dropped, original value should be restored
        assert_eq!(std::env::var(key).unwrap(), "original");

        // Cleanup
        std::env::remove_var(key);
    }

    #[test]
    fn test_database_url_generation() {
        let url = test_database_url("test_module::test_function");
        assert!(url.starts_with("postgres://"));
        assert!(url.contains("resonance_test_"));
        assert!(!url.contains("::"));
    }

    #[test]
    fn test_random_port_is_unique() {
        let port1 = random_test_port();
        let port2 = random_test_port();
        assert_ne!(port1, port2);
    }

    #[tokio::test]
    async fn test_wait_for_success() {
        let result = wait_for(|| async { true }, 100, 10).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_for_timeout() {
        let result = wait_for(|| async { false }, 50, 10).await;
        assert!(!result);
    }
}
