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

    fn create_test_entry(name: &str, env: &str) -> ServiceEntry {
        let mut tags = HashMap::new();
        tags.insert("type".to_string(), "test".to_string());

        ServiceEntry::new(
            name.to_string(),
            env.to_string(),
            crate::model::service_address::ServiceAddress::from_string(format!(
                "http://{}_{}.example.com",
                name, env
            )),
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
}
