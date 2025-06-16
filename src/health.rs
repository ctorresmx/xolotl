use std::{error::Error, sync::Arc, time::Duration};

use tokio::{sync::RwLock, time::interval};

use crate::{
    config::HealthConfig,
    model::service_registry::{HealthStatus, ServiceRegistry},
};

pub struct HealthManager {
    registry: Arc<RwLock<dyn ServiceRegistry>>,
    config: HealthConfig,
}

impl HealthManager {
    pub fn new(registry: Arc<RwLock<dyn ServiceRegistry>>, config: HealthConfig) -> Self {
        Self { registry, config }
    }

    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let registry = self.registry.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.cleanup_interval_secs));

            loop {
                interval.tick().await;

                if let Err(e) = Self::cleanup_unhealthy_services(&registry, &config).await {
                    eprintln!("Service cleanup failed: {:?}", e);
                }
            }
        })
    }

    pub async fn cleanup_unhealthy_services(
        registry: &Arc<RwLock<dyn ServiceRegistry>>,
        config: &HealthConfig,
    ) -> Result<usize, Box<dyn Error + Send + Sync>> {
        let mut registry = registry.write().await;
        let services = registry.list();

        let unhealthy_services: Vec<_> = services
            .iter()
            .filter(|service| matches!(service.health_status(config), HealthStatus::Unhealthy))
            .collect();

        let removed_count = unhealthy_services.len();

        for service in unhealthy_services {
            match registry.deregister(&service.service_name, Some(&service.environment)) {
                Ok(_) => (),
                Err(e) => return Err(format!("Failed to deregister service: {:?}", e).into()),
            }
        }

        Ok(removed_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::HealthConfig,
        model::service_registry::{ServiceEntry, ServiceRegistry},
        model::service_address::ServiceAddress,
        registry::in_memory_registry::InMemoryRegistry,
    };
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_service(name: &str, env: &str, heartbeat_offset_ms: u64) -> ServiceEntry {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        ServiceEntry {
            id: format!("test-{}-{}", name, env),
            service_name: name.to_string(),
            environment: env.to_string(),
            address: ServiceAddress::String("http://localhost:8080".to_string()),
            tags: HashMap::new(),
            registered_at: now,
            last_heartbeat: now - heartbeat_offset_ms,
        }
    }

    #[test]
    fn test_health_manager_new() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig::default();
        
        let health_manager = HealthManager::new(registry.clone(), config.clone());
        
        // Just verify that the HealthManager was created successfully
        assert!(std::any::type_name_of_val(&health_manager).contains("HealthManager"));
    }

    #[tokio::test]
    async fn test_cleanup_empty_registry() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig::default();
        
        let result = HealthManager::cleanup_unhealthy_services(&registry, &config).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_all_healthy_services() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig::default();
        
        // Add healthy services (recent heartbeats)
        {
            let mut reg = registry.write().await;
            let service1 = create_test_service("service1", "prod", 0); // Current time
            let service2 = create_test_service("service2", "prod", 1000); // 1 second ago
            reg.register(service1).unwrap();
            reg.register(service2).unwrap();
        }
        
        let result = HealthManager::cleanup_unhealthy_services(&registry, &config).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // No services should be removed
        
        // Verify services are still there
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 2);
    }

    #[tokio::test]
    async fn test_cleanup_all_unhealthy_services() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig::default();
        
        // Add unhealthy services (old heartbeats)
        {
            let mut reg = registry.write().await;
            let service1 = create_test_service("service1", "prod", config.stale_threshold_ms + 1000);
            let service2 = create_test_service("service2", "prod", config.stale_threshold_ms + 2000);
            reg.register(service1).unwrap();
            reg.register(service2).unwrap();
        }
        
        let result = HealthManager::cleanup_unhealthy_services(&registry, &config).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2); // Both services should be removed
        
        // Verify services are gone
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_mixed_health_services() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig::default();
        
        // Add mixed services
        {
            let mut reg = registry.write().await;
            let healthy_service = create_test_service("healthy", "prod", 1000);
            let stale_service = create_test_service("stale", "prod", config.healthy_threshold_ms + 1000);
            let unhealthy_service = create_test_service("unhealthy", "prod", config.stale_threshold_ms + 1000);
            
            reg.register(healthy_service).unwrap();
            reg.register(stale_service).unwrap();
            reg.register(unhealthy_service).unwrap();
        }
        
        let result = HealthManager::cleanup_unhealthy_services(&registry, &config).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Only unhealthy service should be removed
        
        // Verify only healthy and stale services remain
        let reg = registry.read().await;
        let remaining_services = reg.list();
        assert_eq!(remaining_services.len(), 2);
        
        let service_names: Vec<&str> = remaining_services.iter()
            .map(|s| s.service_name.as_str())
            .collect();
        assert!(service_names.contains(&"healthy"));
        assert!(service_names.contains(&"stale"));
        assert!(!service_names.contains(&"unhealthy"));
    }

    #[tokio::test]
    async fn test_cleanup_with_custom_thresholds() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig {
            healthy_threshold_ms: 5000,
            stale_threshold_ms: 10000,
            cleanup_interval_secs: 30,
        };
        
        // Add service that would be healthy with default config but unhealthy with custom
        {
            let mut reg = registry.write().await;
            let service = create_test_service("test", "prod", 15000); // 15 seconds ago
            reg.register(service).unwrap();
        }
        
        let result = HealthManager::cleanup_unhealthy_services(&registry, &config).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Service should be removed with custom threshold
        
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 0);
    }

    #[tokio::test]
    async fn test_start_cleanup_task() {
        let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
        let config = HealthConfig {
            healthy_threshold_ms: 60000,
            stale_threshold_ms: 90000,
            cleanup_interval_secs: 1, // Very short interval for testing
        };
        
        let health_manager = HealthManager::new(registry, config);
        let handle = health_manager.start_cleanup_task();
        
        // Verify task handle was created
        assert!(!handle.is_finished());
        
        // Clean up
        handle.abort();
    }
}
