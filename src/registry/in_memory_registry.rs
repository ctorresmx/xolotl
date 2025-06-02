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

    pub fn generate_key(name: &str, environment: &str) -> String {
        format!("{}:{}", name, environment)
    }
}

impl ServiceRegistry for InMemoryRegistry {
    fn register(&mut self, entry: ServiceEntry) -> Result<(), RegistryError> {
        let key = Self::generate_key(&entry.name, &entry.environment);
        if self.services.contains_key(&key) {
            return Err(RegistryError::AlreadyExists);
        }

        self.services.insert(key, entry);
        Ok(())
    }

    fn resolve(&self, name: &str, environment: &str) -> Option<ServiceEntry> {
        let key = Self::generate_key(name, environment);
        self.services.get(&key).cloned()
    }

    fn deregister(&mut self, name: &str, environment: Option<&str>) -> Result<(), RegistryError> {
        if let Some(env) = environment {
            // Remove a specific service entry
            let key = Self::generate_key(name, env);
            if self.services.remove(&key).is_some() {
                Ok(())
            } else {
                Err(RegistryError::NotFound)
            }
        } else {
            // Remove all entries for the given name across all environments
            let keys_to_remove: Vec<String> = self
                .services
                .keys()
                .filter(|k| k.starts_with(&format!("{}:", name)))
                .cloned()
                .collect();

            if keys_to_remove.is_empty() {
                return Err(RegistryError::NotFound);
            }

            for key in keys_to_remove {
                self.services.remove(&key);
            }

            Ok(())
        }
    }

    fn list(&self) -> Vec<ServiceEntry> {
        self.services.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(name: &str, env: &str) -> ServiceEntry {
        ServiceEntry {
            name: name.to_string(),
            environment: env.to_string(),
            address: format!("http://{}_{}.example.com", name, env),
            tags: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_generate_key() {
        assert_eq!(
            InMemoryRegistry::generate_key("service1", "dev"),
            "service1:dev"
        );
    }

    #[test]
    fn test_register_success() {
        let mut registry = InMemoryRegistry::new();
        let entry = create_test_entry("service1", "dev");

        let result = registry.register(entry.clone());
        assert!(result.is_ok());

        // Verify the entry was stored
        let stored = registry.resolve(&entry.name, &entry.environment);
        assert!(stored.is_some());
        let stored = stored.unwrap();
        assert_eq!(stored.name, "service1");
        assert_eq!(stored.environment, "dev");
        assert_eq!(stored.address, "http://service1_dev.example.com");
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
    fn test_resolve_found() {
        let mut registry = InMemoryRegistry::new();
        let entry = create_test_entry("service1", "dev");

        registry.register(entry.clone()).unwrap();

        let result = registry.resolve("service1", "dev");
        assert!(result.is_some());
        let resolved = result.unwrap();
        assert_eq!(resolved.name, "service1");
        assert_eq!(resolved.environment, "dev");
    }

    #[test]
    fn test_resolve_not_found() {
        let registry = InMemoryRegistry::new();

        let result = registry.resolve("nonexistent", "dev");
        assert!(result.is_none());
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
        assert!(registry.resolve("service1", "dev").is_none());
        assert!(registry.resolve("service1", "prod").is_some());
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
        assert!(registry.resolve("service1", "dev").is_none());
        assert!(registry.resolve("service1", "prod").is_none());

        // Verify service2 still exists
        assert!(registry.resolve("service2", "dev").is_some());
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
        let names: Vec<String> = services.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"service1".to_string()));
        assert!(names.contains(&"service2".to_string()));

        let envs: Vec<String> = services.iter().map(|s| s.environment.clone()).collect();
        assert!(envs.contains(&"dev".to_string()));
        assert!(envs.contains(&"prod".to_string()));
    }
}
