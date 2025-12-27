//! Health check service for verifying external dependencies
//!
//! This service provides health checks for all critical infrastructure:
//! - PostgreSQL database
//! - Redis cache
//! - Meilisearch search engine
//! - Ollama AI service

use serde::Serialize;
use std::time::{Duration, Instant};

/// Status of an individual service
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    /// Service is healthy and responding
    Healthy,
    /// Service is unhealthy or unreachable
    Unhealthy,
    /// Service check was skipped (e.g., optional service not configured)
    Skipped,
}

/// Result of a single service health check
#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealth {
    /// Name of the service
    pub name: &'static str,
    /// Current status
    pub status: ServiceStatus,
    /// Response time in milliseconds (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,
    /// Error message if unhealthy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional details about the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ServiceHealth {
    /// Create a healthy service result
    pub fn healthy(name: &'static str, response_time: Duration) -> Self {
        Self {
            name,
            status: ServiceStatus::Healthy,
            response_time_ms: Some(response_time.as_millis() as u64),
            error: None,
            details: None,
        }
    }

    /// Create a healthy service result with details
    pub fn healthy_with_details(
        name: &'static str,
        response_time: Duration,
        details: serde_json::Value,
    ) -> Self {
        Self {
            name,
            status: ServiceStatus::Healthy,
            response_time_ms: Some(response_time.as_millis() as u64),
            error: None,
            details: Some(details),
        }
    }

    /// Create an unhealthy service result
    pub fn unhealthy(name: &'static str, error: impl Into<String>) -> Self {
        Self {
            name,
            status: ServiceStatus::Unhealthy,
            response_time_ms: None,
            error: Some(error.into()),
            details: None,
        }
    }

    /// Create an unhealthy service result with response time
    pub fn unhealthy_with_time(
        name: &'static str,
        error: impl Into<String>,
        response_time: Duration,
    ) -> Self {
        Self {
            name,
            status: ServiceStatus::Unhealthy,
            response_time_ms: Some(response_time.as_millis() as u64),
            error: Some(error.into()),
            details: None,
        }
    }

    /// Create a skipped service result (for optional services not configured)
    #[allow(dead_code)]
    pub fn skipped(name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            name,
            status: ServiceStatus::Skipped,
            response_time_ms: None,
            error: None,
            details: Some(serde_json::json!({ "reason": reason.into() })),
        }
    }
}

/// Aggregated health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckResponse {
    /// Overall status (healthy only if all required services are healthy)
    pub status: ServiceStatus,
    /// Individual service health results
    pub services: Vec<ServiceHealth>,
    /// Total time to complete all health checks
    pub total_time_ms: u64,
    /// API version
    pub version: &'static str,
}

impl HealthCheckResponse {
    /// Create a new health check response from individual service results
    pub fn new(services: Vec<ServiceHealth>, total_time: Duration) -> Self {
        let status = if services
            .iter()
            .all(|s| s.status == ServiceStatus::Healthy || s.status == ServiceStatus::Skipped)
        {
            ServiceStatus::Healthy
        } else {
            ServiceStatus::Unhealthy
        };

        Self {
            status,
            services,
            total_time_ms: total_time.as_millis() as u64,
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Check if overall health is good
    pub fn is_healthy(&self) -> bool {
        self.status == ServiceStatus::Healthy
    }
}

/// Health check service for verifying external dependencies
pub struct HealthService {
    http_client: reqwest::Client,
}

impl HealthService {
    /// Create a new health service
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Check PostgreSQL database connectivity
    pub async fn check_database(&self, database_url: &str) -> ServiceHealth {
        let start = Instant::now();

        match sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
        {
            Ok(pool) => {
                // Run a simple query to verify the connection works
                match sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
                {
                    Ok(_) => {
                        let elapsed = start.elapsed();
                        // Try to get database version for details
                        let version = sqlx::query_scalar::<_, String>("SELECT version()")
                            .fetch_optional(&pool)
                            .await
                            .ok()
                            .flatten();

                        let details = version.map(|v| serde_json::json!({ "version": v }));

                        if let Some(details) = details {
                            ServiceHealth::healthy_with_details("database", elapsed, details)
                        } else {
                            ServiceHealth::healthy("database", elapsed)
                        }
                    }
                    Err(e) => ServiceHealth::unhealthy_with_time(
                        "database",
                        format!("Query failed: {}", e),
                        start.elapsed(),
                    ),
                }
            }
            Err(e) => ServiceHealth::unhealthy("database", format!("Connection failed: {}", e)),
        }
    }

    /// Check Redis connectivity
    pub async fn check_redis(&self, redis_url: &str) -> ServiceHealth {
        let start = Instant::now();

        match redis::Client::open(redis_url) {
            Ok(client) => {
                match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        // Run a PING command to verify the connection
                        match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                            Ok(response) => {
                                let elapsed = start.elapsed();
                                if response == "PONG" {
                                    // Get Redis info for details
                                    let info: Option<String> = redis::cmd("INFO")
                                        .arg("server")
                                        .query_async::<_, String>(&mut conn)
                                        .await
                                        .ok();

                                    let version = info.and_then(|info| {
                                        info.lines()
                                            .find(|line| line.starts_with("redis_version:"))
                                            .map(|line| {
                                                line.trim_start_matches("redis_version:")
                                                    .to_string()
                                            })
                                    });

                                    let details =
                                        version.map(|v| serde_json::json!({ "version": v }));

                                    if let Some(details) = details {
                                        ServiceHealth::healthy_with_details(
                                            "redis", elapsed, details,
                                        )
                                    } else {
                                        ServiceHealth::healthy("redis", elapsed)
                                    }
                                } else {
                                    ServiceHealth::unhealthy_with_time(
                                        "redis",
                                        format!("Unexpected PING response: {}", response),
                                        elapsed,
                                    )
                                }
                            }
                            Err(e) => ServiceHealth::unhealthy_with_time(
                                "redis",
                                format!("PING failed: {}", e),
                                start.elapsed(),
                            ),
                        }
                    }
                    Err(e) => {
                        ServiceHealth::unhealthy("redis", format!("Connection failed: {}", e))
                    }
                }
            }
            Err(e) => ServiceHealth::unhealthy("redis", format!("Invalid URL: {}", e)),
        }
    }

    /// Check Meilisearch connectivity
    pub async fn check_meilisearch(&self, url: &str, api_key: &str) -> ServiceHealth {
        let start = Instant::now();

        // Meilisearch health endpoint: GET /health
        let health_url = format!("{}/health", url.trim_end_matches('/'));

        match self
            .http_client
            .get(&health_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
        {
            Ok(response) => {
                let elapsed = start.elapsed();
                if response.status().is_success() {
                    // Try to get version info
                    let version_url = format!("{}/version", url.trim_end_matches('/'));
                    let version_info: Option<serde_json::Value> = match self
                        .http_client
                        .get(&version_url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .send()
                        .await
                    {
                        Ok(r) if r.status().is_success() => r.json().await.ok(),
                        _ => None,
                    };

                    if let Some(info) = version_info {
                        ServiceHealth::healthy_with_details("meilisearch", elapsed, info)
                    } else {
                        ServiceHealth::healthy("meilisearch", elapsed)
                    }
                } else {
                    ServiceHealth::unhealthy_with_time(
                        "meilisearch",
                        format!("Unhealthy status: {}", response.status()),
                        elapsed,
                    )
                }
            }
            Err(e) => ServiceHealth::unhealthy("meilisearch", format!("Request failed: {}", e)),
        }
    }

    /// Check Ollama AI service connectivity
    pub async fn check_ollama(&self, url: &str, model: &str) -> ServiceHealth {
        let start = Instant::now();

        // Ollama API endpoint: GET /api/tags (list models)
        let tags_url = format!("{}/api/tags", url.trim_end_matches('/'));

        match self.http_client.get(&tags_url).send().await {
            Ok(response) => {
                let elapsed = start.elapsed();
                if response.status().is_success() {
                    // Parse the response to check if the configured model is available
                    match response.json::<serde_json::Value>().await {
                        Ok(data) => {
                            let models = data
                                .get("models")
                                .and_then(|m| m.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                                        .map(String::from)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();

                            let model_available = models
                                .iter()
                                .any(|m| m == model || m.starts_with(&format!("{}:", model)));

                            let details = serde_json::json!({
                                "configured_model": model,
                                "model_available": model_available,
                                "available_models": models.len(),
                            });

                            if model_available {
                                ServiceHealth::healthy_with_details("ollama", elapsed, details)
                            } else {
                                // Still healthy, but note that model is not available
                                ServiceHealth {
                                    name: "ollama",
                                    status: ServiceStatus::Healthy,
                                    response_time_ms: Some(elapsed.as_millis() as u64),
                                    error: Some(format!(
                                        "Configured model '{}' not found. Available: {}",
                                        model,
                                        models.join(", ")
                                    )),
                                    details: Some(details),
                                }
                            }
                        }
                        Err(e) => ServiceHealth::unhealthy_with_time(
                            "ollama",
                            format!("Failed to parse response: {}", e),
                            elapsed,
                        ),
                    }
                } else {
                    ServiceHealth::unhealthy_with_time(
                        "ollama",
                        format!("Unhealthy status: {}", response.status()),
                        elapsed,
                    )
                }
            }
            Err(e) => ServiceHealth::unhealthy("ollama", format!("Request failed: {}", e)),
        }
    }

    /// Run all health checks in parallel
    pub async fn check_all(
        &self,
        database_url: &str,
        redis_url: &str,
        meilisearch_url: &str,
        meilisearch_key: &str,
        ollama_url: &str,
        ollama_model: &str,
    ) -> HealthCheckResponse {
        let start = Instant::now();

        // Run all checks in parallel using tokio::join!
        let (db_health, redis_health, meili_health, ollama_health) = tokio::join!(
            self.check_database(database_url),
            self.check_redis(redis_url),
            self.check_meilisearch(meilisearch_url, meilisearch_key),
            self.check_ollama(ollama_url, ollama_model),
        );

        let services = vec![db_health, redis_health, meili_health, ollama_health];

        HealthCheckResponse::new(services, start.elapsed())
    }
}

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_health_healthy() {
        let health = ServiceHealth::healthy("test", Duration::from_millis(50));
        assert_eq!(health.status, ServiceStatus::Healthy);
        assert_eq!(health.response_time_ms, Some(50));
        assert!(health.error.is_none());
    }

    #[test]
    fn test_service_health_unhealthy() {
        let health = ServiceHealth::unhealthy("test", "Connection refused");
        assert_eq!(health.status, ServiceStatus::Unhealthy);
        assert!(health.response_time_ms.is_none());
        assert_eq!(health.error, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_service_health_skipped() {
        let health = ServiceHealth::skipped("test", "Not configured");
        assert_eq!(health.status, ServiceStatus::Skipped);
    }

    #[test]
    fn test_health_check_response_all_healthy() {
        let services = vec![
            ServiceHealth::healthy("db", Duration::from_millis(10)),
            ServiceHealth::healthy("redis", Duration::from_millis(5)),
        ];
        let response = HealthCheckResponse::new(services, Duration::from_millis(15));
        assert!(response.is_healthy());
        assert_eq!(response.status, ServiceStatus::Healthy);
    }

    #[test]
    fn test_health_check_response_one_unhealthy() {
        let services = vec![
            ServiceHealth::healthy("db", Duration::from_millis(10)),
            ServiceHealth::unhealthy("redis", "Connection refused"),
        ];
        let response = HealthCheckResponse::new(services, Duration::from_millis(15));
        assert!(!response.is_healthy());
        assert_eq!(response.status, ServiceStatus::Unhealthy);
    }

    #[test]
    fn test_health_check_response_with_skipped() {
        let services = vec![
            ServiceHealth::healthy("db", Duration::from_millis(10)),
            ServiceHealth::skipped("optional", "Not configured"),
        ];
        let response = HealthCheckResponse::new(services, Duration::from_millis(15));
        assert!(response.is_healthy());
    }
}
