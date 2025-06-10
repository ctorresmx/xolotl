use crate::model::service_registry::{RegistryError, ServiceEntry, ServiceRegistry};
use std::collections::HashMap;

pub struct InMemoryRegistry {
    services: HashMap<String, ServiceEntry>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        InMemoryRegistry {
            services: HashMap::new(),
        }
    }
}

impl ServiceRegistry for InMemoryRegistry {
    fn list(&self) -> Vec<ServiceEntry> {
        self.services.values().cloned().collect()
    }

    fn register(&mut self, entry: ServiceEntry) -> Result<(), RegistryError> {
        if self.services.contains_key(&entry.id) {
            return Err(RegistryError::AlreadyExists);
        }

        self.services.insert(entry.id.clone(), entry);
        Ok(())
    }

    fn resolve(&self, service_name: &str, environment: &str) -> Vec<ServiceEntry> {
        self.services
            .values()
            .filter(|service| {
                service.service_name == service_name && service.environment == environment
            })
            .cloned()
            .collect()
    }

    fn deregister(
        &mut self,
        service_name: &str,
        environment: Option<&str>,
    ) -> Result<(), RegistryError> {
        let ids_to_remove: Vec<String> = if let Some(env) = environment {
            // Remove services matching specific service name and environment
            self.services
                .iter()
                .filter(|(_, service)| {
                    service.service_name == service_name && service.environment == env
                })
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            // Remove all services matching the service name across all environments
            self.services
                .iter()
                .filter(|(_, service)| service.service_name == service_name)
                .map(|(id, _)| id.clone())
                .collect()
        };

        if ids_to_remove.is_empty() {
            return Err(RegistryError::NotFound);
        }

        for id in ids_to_remove {
            self.services.remove(&id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_entry(name: &str, env: &str) -> ServiceEntry {
        let mut tags = HashMap::new();
        tags.insert("type".to_string(), "test".to_string());

        ServiceEntry::new(
            name.to_string(),
            env.to_string(),
            format!("http://{}_{}.example.com", name, env),
            tags,
        )
    }

    #[test]
    fn test_register_success() {
        let mut registry = InMemoryRegistry::new();
        let entry = create_test_entry("service1", "dev");

        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        // Verify the entry was stored
        let stored = registry.resolve(&entry.service_name, &entry.environment);
        assert_eq!(stored.len(), 1);
        let stored = &stored[0];
        assert_eq!(stored.service_name, "service1");
        assert_eq!(stored.environment, "dev");
        assert_eq!(stored.address_str(), "http://service1_dev.example.com");
    }

    #[test]
    fn test_register_duplicate() {
        let mut registry = InMemoryRegistry::new();
        let entry = create_test_entry("service1", "dev");

        // Register once successfully
        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        // Try to register again with the same name and environment
        let result = registry.register(entry);
        assert!(result.is_err());
        match result {
            Err(RegistryError::AlreadyExists) => {}
            _ => panic!("Expected AlreadyExists error"),
        }
    }

    #[test]
    fn test_register_same_uuid_twice() {
        let mut registry = InMemoryRegistry::new();

        // Create an entry manually
        let entry = create_test_entry("service1", "dev");

        // Register first time
        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        // Try to register the exact same entry (same UUID) - should fail
        let result = registry.register(entry);
        assert!(result.is_err());
        match result {
            Err(RegistryError::AlreadyExists) => {}
            _ => panic!("Expected AlreadyExists error"),
        }
    }

    #[test]
    fn test_resolve_found() {
        let mut registry = InMemoryRegistry::new();
        let entry = create_test_entry("service1", "dev");

        registry.register(entry.clone()).unwrap();

        let result = registry.resolve("service1", "dev");
        assert_eq!(result.len(), 1);
        let resolved = &result[0];
        assert_eq!(resolved.service_name, "service1");
        assert_eq!(resolved.environment, "dev");
    }

    #[test]
    fn test_resolve_not_found() {
        let registry = InMemoryRegistry::new();

        let result = registry.resolve("nonexistent", "dev");
        assert!(result.is_empty());
    }

    #[test]
    fn test_deregister_specific_environment() {
        let mut registry = InMemoryRegistry::new();

        // Register services
        registry
            .register(create_test_entry("service1", "dev"))
            .unwrap();
        registry
            .register(create_test_entry("service1", "prod"))
            .unwrap();

        // Deregister specific environment
        let result = registry.deregister("service1", Some("dev"));
        assert!(result.is_ok());

        // Verify only the dev environment was removed
        assert!(registry.resolve("service1", "dev").is_empty());
        assert_eq!(registry.resolve("service1", "prod").len(), 1);
    }

    #[test]
    fn test_deregister_all_environments() {
        let mut registry = InMemoryRegistry::new();

        // Register services
        registry
            .register(create_test_entry("service1", "dev"))
            .unwrap();
        registry
            .register(create_test_entry("service1", "prod"))
            .unwrap();
        registry
            .register(create_test_entry("service2", "dev"))
            .unwrap();

        // Deregister all environments for service1
        let result = registry.deregister("service1", None);
        assert!(result.is_ok());

        // Verify all service1 entries were removed
        assert!(registry.resolve("service1", "dev").is_empty());
        assert!(registry.resolve("service1", "prod").is_empty());

        // Verify service2 still exists
        assert_eq!(registry.resolve("service2", "dev").len(), 1);
    }

    #[test]
    fn test_deregister_not_found() {
        let mut registry = InMemoryRegistry::new();

        // Try to deregister a service that doesn't exist
        let result = registry.deregister("nonexistent", Some("dev"));
        assert!(result.is_err());
        match result {
            Err(RegistryError::NotFound) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_list_empty() {
        let registry = InMemoryRegistry::new();

        let services = registry.list();
        assert!(services.is_empty());
    }

    #[test]
    fn test_list_with_entries() {
        let mut registry = InMemoryRegistry::new();

        // Register several services
        registry
            .register(create_test_entry("service1", "dev"))
            .unwrap();
        registry
            .register(create_test_entry("service1", "prod"))
            .unwrap();
        registry
            .register(create_test_entry("service2", "dev"))
            .unwrap();

        // List all services
        let services = registry.list();
        assert_eq!(services.len(), 3);

        // Verify all expected services are in the list
        let names: Vec<String> = services.iter().map(|s| s.service_name.clone()).collect();
        assert!(names.contains(&"service1".to_string()));
        assert!(names.contains(&"service2".to_string()));

        let envs: Vec<String> = services.iter().map(|s| s.environment.clone()).collect();
        assert!(envs.contains(&"dev".to_string()));
        assert!(envs.contains(&"prod".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_registry_operations() {
        let registry = Arc::new(RwLock::new(InMemoryRegistry::new()));

        // Spawn multiple concurrent tasks
        let mut handles = vec![];

        // Register services concurrently
        for i in 0..10 {
            let registry_clone = registry.clone();
            let handle = tokio::spawn(async move {
                let mut reg = registry_clone.write().await;
                let entry = create_test_entry(&format!("service{}", i), "dev");
                reg.register(entry)
            });
            handles.push(handle);
        }

        // Wait for all registrations to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 10);

        // Verify all services were registered
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 10);
    }

    #[test]
    fn test_registry_with_special_characters() {
        let mut registry = InMemoryRegistry::new();

        // Test with special characters in service names and environments
        let mut tags = HashMap::new();
        tags.insert("special-key".to_string(), "special@value#123".to_string());

        let entry = ServiceEntry::new(
            "service-with-dashes_and_underscores".to_string(),
            "dev-environment_v1.2".to_string(),
            "http://my-service.example.com:8080/api/v1".to_string(),
            tags,
        );

        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        let resolved = registry.resolve(&entry.service_name, &entry.environment);
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0].service_name,
            "service-with-dashes_and_underscores"
        );
        assert_eq!(resolved[0].environment, "dev-environment_v1.2");
    }

    #[test]
    fn test_registry_empty_tags() {
        let mut registry = InMemoryRegistry::new();

        let entry = ServiceEntry::new(
            "no-tags-service".to_string(),
            "prod".to_string(),
            "http://simple.example.com".to_string(),
            HashMap::new(),
        );

        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        let resolved = registry.resolve(&entry.service_name, &entry.environment);
        assert_eq!(resolved.len(), 1);
        assert!(resolved[0].tags.is_empty());
    }

    #[test]
    fn test_registry_unicode_values() {
        let mut registry = InMemoryRegistry::new();

        let mut tags = HashMap::new();
        tags.insert("description".to_string(), "服务描述".to_string());
        tags.insert("owner".to_string(), "José María".to_string());

        let entry = ServiceEntry::new(
            "unicode-service".to_string(),
            "测试环境".to_string(),
            "http://unicode.example.com".to_string(),
            tags.clone(),
        );

        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        let resolved = registry.resolve(&entry.service_name, &entry.environment);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].tags.get("description").unwrap(), "服务描述");
        assert_eq!(resolved[0].tags.get("owner").unwrap(), "José María");
    }

    #[test]
    fn test_deregister_partial_matches() {
        let mut registry = InMemoryRegistry::new();

        // Register services with similar names
        registry
            .register(create_test_entry("service", "dev"))
            .unwrap();
        registry
            .register(create_test_entry("service1", "dev"))
            .unwrap();
        registry
            .register(create_test_entry("service-extended", "dev"))
            .unwrap();

        // Deregister only "service" - should not affect others
        let result = registry.deregister("service", Some("dev"));
        assert!(result.is_ok());

        // Verify only the exact match was removed
        assert!(registry.resolve("service", "dev").is_empty());
        assert_eq!(registry.resolve("service1", "dev").len(), 1);
        assert_eq!(registry.resolve("service-extended", "dev").len(), 1);
    }
}
