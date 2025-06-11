use crate::model::service_address::ServiceAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        let registered_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Generation of registered_at timestamp failed")
            .as_millis() as u64;

        ServiceEntry {
            id,
            service_name,
            environment,
            address: ServiceAddress::String(address),
            tags,
            registered_at,
        }
    }

    /// Returns the address as a string reference
    pub fn address_str(&self) -> &str {
        self.address.as_str()
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
    fn test_registry_error_variants() {
        // Test all error variants to ensure they work correctly
        let already_exists = RegistryError::AlreadyExists;
        let not_found = RegistryError::NotFound;
        let internal_error = RegistryError::InternalError("Internal error".to_string());

        // Verify they have different discriminants
        assert!(matches!(already_exists, RegistryError::AlreadyExists));
        assert!(matches!(not_found, RegistryError::NotFound));
        assert!(matches!(internal_error, RegistryError::InternalError(_)));
    }
}
