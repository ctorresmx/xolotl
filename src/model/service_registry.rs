use crate::config::HealthConfig;
use crate::model::service_address::ServiceAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    pub id: String,
    pub service_name: String,
    pub environment: String,
    pub address: ServiceAddress,
    pub tags: HashMap<String, String>,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Generation of current timestamp failed")
        .as_millis() as u64
}

impl ServiceEntry {
    /// Creates a new ServiceEntry with auto-generated UUID and timestamp
    pub fn new(
        service_name: String,
        environment: String,
        address: String,
        tags: HashMap<String, String>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let registered_at = now();

        ServiceEntry {
            id,
            service_name,
            environment,
            address: ServiceAddress::String(address),
            tags,
            registered_at,
            last_heartbeat: registered_at, // This is a new entry so let's set heartbeat to the creation time
        }
    }

    /// Returns the address as a string reference
    pub fn address_str(&self) -> &str {
        self.address.as_str()
    }

    pub fn health_status(&self, config: &HealthConfig) -> HealthStatus {
        match self.time_since_last_heartbeat() {
            0 => HealthStatus::Unknown,
            time if time <= config.healthy_threshold_ms => HealthStatus::Healthy,
            time if time <= config.stale_threshold_ms => HealthStatus::Stale,
            _ => HealthStatus::Unhealthy,
        }
    }

    /// Returns the time elapsed since the last heartbeat in millis
    #[allow(dead_code)]
    pub fn time_since_last_heartbeat(&self) -> u64 {
        now() - self.last_heartbeat
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unknown,   // Maybe just registered without heartbeat
    Stale,     // Missed heartbeat but still within timeout
    Unhealthy, // No heartbeat and will be cleaned up
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Unknown => write!(f, "Unknown"),
            HealthStatus::Stale => write!(f, "Stale"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

pub trait ServiceRegistry: Sync + Send + 'static {
    fn list(&self) -> Vec<ServiceEntry>;
    fn register(&mut self, entry: ServiceEntry) -> Result<(), RegistryError>;
    fn resolve(&self, service_name: &str, environment: &str) -> Vec<ServiceEntry>;
    fn deregister(
        &mut self,
        service_name: &str,
        environment: Option<&str>,
    ) -> Result<(), RegistryError>;
    fn heartbeat(&mut self, service_name: &str, environment: &str) -> Result<(), RegistryError>;
}

#[derive(Debug)]
pub enum RegistryError {
    AlreadyExists,
    NotFound,
    #[allow(dead_code)]
    InternalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_new_service_entry() {
        let mut tags = HashMap::new();
        tags.insert("type".to_string(), "api".to_string());
        tags.insert("version".to_string(), "v1".to_string());

        let entry = ServiceEntry::new(
            "my-service".to_string(),
            "production".to_string(),
            "https://api.example.com:443".to_string(),
            tags.clone(),
        );

        // Check fields are properly set
        assert!(!entry.id.is_empty()); // UUID should be set
        assert_eq!(entry.service_name, "my-service");
        assert_eq!(entry.environment, "production");
        assert_eq!(entry.address_str(), "https://api.example.com:443");
        assert_eq!(entry.tags, tags);
        assert!(entry.registered_at > 0); // Timestamp should be set
        assert!(matches!(
            entry.health_status(&HealthConfig::default()),
            HealthStatus::Unknown
        ));
        assert_eq!(entry.last_heartbeat, entry.registered_at); // Last heartbeat should be equal to the creation time

        // Check that we're using millisecond precision (timestamp should be much larger than a seconds-based one)
        assert!(
            entry.registered_at > 1_000_000_000_000,
            "Timestamp should be in milliseconds"
        );
    }

    #[test]
    fn test_address_str() {
        let mut tags = HashMap::new();
        tags.insert("type".to_string(), "api".to_string());

        let entry = ServiceEntry::new(
            "my-service".to_string(),
            "production".to_string(),
            "https://api.example.com:443".to_string(),
            tags,
        );

        assert_eq!(entry.address_str(), "https://api.example.com:443");
        assert_eq!(entry.address_str(), entry.address.as_str());
    }

    #[test]
    fn test_registry_error_internal_error() {
        let error = RegistryError::InternalError("Database connection failed".to_string());

        // Test that we can match on it (ensures it's not dead code)
        match error {
            RegistryError::InternalError(msg) => {
                assert_eq!(msg, "Database connection failed");
            }
            _ => panic!("Expected InternalError variant"),
        }

        // Test Debug formatting
        let error = RegistryError::InternalError("Test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("InternalError"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_health_status_with_default_config() {
        let config = HealthConfig::default();
        let mut entry = ServiceEntry::new(
            "test-service".to_string(),
            "prod".to_string(),
            "http://test.example.com".to_string(),
            HashMap::new(),
        );

        // Test Unknown status (last_heartbeat == registered_at)
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Unknown
        ));

        // Test Healthy status (within healthy threshold)
        entry.last_heartbeat = now() - 30000; // 30 seconds ago
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Healthy
        ));

        // Test Stale status (beyond healthy but within stale threshold)
        entry.last_heartbeat = now() - 75000; // 75 seconds ago
        assert!(matches!(entry.health_status(&config), HealthStatus::Stale));

        // Test Unhealthy status (beyond stale threshold)
        entry.last_heartbeat = now() - 120000; // 120 seconds ago
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Unhealthy
        ));
    }

    #[test]
    fn test_health_status_with_custom_config() {
        let config = HealthConfig {
            healthy_threshold_ms: 10000, // 10 seconds
            stale_threshold_ms: 20000,   // 20 seconds
            cleanup_interval_secs: 30,
        };

        let mut entry = ServiceEntry::new(
            "test-service".to_string(),
            "prod".to_string(),
            "http://test.example.com".to_string(),
            HashMap::new(),
        );

        // Test Healthy status (within custom healthy threshold)
        entry.last_heartbeat = now() - 5000; // 5 seconds ago
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Healthy
        ));

        // Test Stale status (beyond healthy but within custom stale threshold)
        entry.last_heartbeat = now() - 15000; // 15 seconds ago
        assert!(matches!(entry.health_status(&config), HealthStatus::Stale));

        // Test Unhealthy status (beyond custom stale threshold)
        entry.last_heartbeat = now() - 25000; // 25 seconds ago
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Unhealthy
        ));
    }

    #[test]
    fn test_health_status_boundary_conditions() {
        let config = HealthConfig {
            healthy_threshold_ms: 10000,
            stale_threshold_ms: 20000,
            cleanup_interval_secs: 30,
        };

        let mut entry = ServiceEntry::new(
            "test-service".to_string(),
            "prod".to_string(),
            "http://test.example.com".to_string(),
            HashMap::new(),
        );

        // Test exact boundary: healthy threshold
        entry.last_heartbeat = now() - 10000; // Exactly 10 seconds ago
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Healthy
        ));

        // Test just past healthy threshold
        entry.last_heartbeat = now() - 10001; // Just over 10 seconds ago
        assert!(matches!(entry.health_status(&config), HealthStatus::Stale));

        // Test exact boundary: stale threshold
        entry.last_heartbeat = now() - 20000; // Exactly 20 seconds ago
        assert!(matches!(entry.health_status(&config), HealthStatus::Stale));

        // Test just past stale threshold
        entry.last_heartbeat = now() - 20001; // Just over 20 seconds ago
        assert!(matches!(
            entry.health_status(&config),
            HealthStatus::Unhealthy
        ));
    }

    #[test]
    fn test_health_status_display_formatting() {
        assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "Unknown");
        assert_eq!(HealthStatus::Stale.to_string(), "Stale");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "Unhealthy");
    }

    #[test]
    fn test_time_since_last_heartbeat() {
        let mut entry = ServiceEntry::new(
            "test-service".to_string(),
            "prod".to_string(),
            "http://test.example.com".to_string(),
            HashMap::new(),
        );

        // Test with current time (should be approximately 0)
        entry.last_heartbeat = now();
        let elapsed = entry.time_since_last_heartbeat();
        assert!(elapsed < 10); // Should be very small, account for execution time

        // Test with past time
        entry.last_heartbeat = now() - 5000; // 5 seconds ago
        let elapsed = entry.time_since_last_heartbeat();
        assert!((4990..=5010).contains(&elapsed)); // Should be around 5000ms, with some tolerance
    }
}
