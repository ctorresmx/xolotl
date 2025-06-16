use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use xolotl::{
    config::HealthConfig,
    health::HealthManager,
    model::{
        service_registry::{ServiceEntry, ServiceRegistry},
    },
    registry::in_memory_registry::InMemoryRegistry,
};

fn create_service_entry(name: &str, env: &str, address: &str) -> ServiceEntry {
    ServiceEntry::new(
        name.to_string(),
        env.to_string(),
        address.to_string(),
        HashMap::new(),
    )
}

fn create_old_service_entry(name: &str, env: &str, address: &str, age_ms: u64) -> ServiceEntry {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    
    let old_time = now - age_ms;
    
    ServiceEntry {
        id: format!("{}-{}", name, env),
        service_name: name.to_string(),
        environment: env.to_string(),
        address: xolotl::model::service_address::ServiceAddress::String(address.to_string()),
        tags: HashMap::new(),
        registered_at: old_time,
        last_heartbeat: old_time,
    }
}

#[tokio::test]
async fn test_health_management_lifecycle() {
    let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
    let config = HealthConfig {
        healthy_threshold_ms: 2000, // 2 seconds
        stale_threshold_ms: 4000,   // 4 seconds
        cleanup_interval_secs: 1,   // 1 second cleanup interval
    };

    // Register initial services with old timestamps (make them unhealthy)
    {
        let mut reg = registry.write().await;
        let service1 = create_old_service_entry("api", "prod", "http://api-1:8080", config.stale_threshold_ms + 1000);
        let service2 = create_old_service_entry("worker", "prod", "http://worker-1:8080", config.stale_threshold_ms + 2000);
        reg.register(service1).unwrap();
        reg.register(service2).unwrap();
    }

    // Verify services are registered
    {
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 2);
    }

    // Start health manager
    let health_manager = HealthManager::new(registry.clone(), config.clone());
    let cleanup_handle = health_manager.start_cleanup_task();

    // Run cleanup manually to test the logic
    let removed_count = HealthManager::cleanup_unhealthy_services(&registry, &config)
        .await
        .unwrap();
    
    assert_eq!(removed_count, 2); // Both services should be cleaned up

    // Verify services are removed
    {
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 0);
    }

    // Clean up the background task
    cleanup_handle.abort();
}

#[tokio::test]
async fn test_service_reregistration_after_cleanup() {
    let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
    let config = HealthConfig {
        healthy_threshold_ms: 1000,
        stale_threshold_ms: 2000,
        cleanup_interval_secs: 1,
    };

    // Register service with old timestamp
    let service_name = "test-service";
    let environment = "staging";
    {
        let mut reg = registry.write().await;
        let service = create_old_service_entry(service_name, environment, "http://test:8080", config.stale_threshold_ms + 500);
        reg.register(service).unwrap();
    }

    // Run cleanup
    let removed_count = HealthManager::cleanup_unhealthy_services(&registry, &config)
        .await
        .unwrap();
    
    assert_eq!(removed_count, 1);

    // Verify service is removed
    {
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 0);
        assert!(reg.resolve(service_name, environment).is_empty());
    }

    // Re-register the same service
    {
        let mut reg = registry.write().await;
        let new_service = create_service_entry(service_name, environment, "http://test-new:8080");
        reg.register(new_service).unwrap();
    }

    // Verify service is available again
    {
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 1);
        let services = reg.resolve(service_name, environment);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].address_str(), "http://test-new:8080");
    }
}

#[tokio::test]
async fn test_concurrent_cleanup_and_registration() {
    let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
    let config = HealthConfig {
        healthy_threshold_ms: 1000,
        stale_threshold_ms: 2000,
        cleanup_interval_secs: 1,
    };

    // Register initial services with old timestamps
    {
        let mut reg = registry.write().await;
        for i in 0..5 {
            let service = create_old_service_entry(
                &format!("service-{}", i), 
                "prod", 
                &format!("http://service-{}:8080", i),
                config.stale_threshold_ms + 500
            );
            reg.register(service).unwrap();
        }
    }

    // Simulate concurrent operations
    let registry_clone = registry.clone();
    let registry_clone2 = registry.clone();
    let config_clone = config.clone();
    
    let cleanup_task = tokio::spawn(async move {
        HealthManager::cleanup_unhealthy_services(&registry_clone, &config_clone).await
    });

    let registration_task = tokio::spawn(async move {
        let mut reg = registry_clone2.write().await;
        let new_service = create_service_entry("new-service", "prod", "http://new:8080");
        reg.register(new_service).unwrap();
        1
    });

    let (cleanup_result, registration_result) = tokio::join!(cleanup_task, registration_task);
    
    assert!(cleanup_result.is_ok());
    assert!(registration_result.is_ok());
    
    let removed_count = cleanup_result.unwrap().unwrap();
    let registered_count = registration_result.unwrap();
    
    assert_eq!(removed_count, 5); // Old services cleaned up
    assert_eq!(registered_count, 1); // New service registered

    // Verify final state
    {
        let reg = registry.read().await;
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].service_name, "new-service");
    }
}

#[tokio::test]
async fn test_health_status_transitions() {
    let registry: Arc<RwLock<dyn ServiceRegistry>> = Arc::new(RwLock::new(InMemoryRegistry::new()));
    let config = HealthConfig {
        healthy_threshold_ms: 1000, // 1 second
        stale_threshold_ms: 2000,   // 2 seconds  
        cleanup_interval_secs: 1,
    };

    // Test different health statuses by creating services with different ages
    let healthy_service_id = {
        let mut reg = registry.write().await;
        let service = create_service_entry("healthy-test", "dev", "http://healthy:8080");
        let id = service.id.clone();
        reg.register(service).unwrap();
        id
    };

    let stale_service_id = {
        let mut reg = registry.write().await;
        let service = create_old_service_entry("stale-test", "dev", "http://stale:8080", config.healthy_threshold_ms + 500);
        let id = service.id.clone();
        reg.register(service).unwrap();
        id
    };

    let unhealthy_service_id = {
        let mut reg = registry.write().await;
        let service = create_old_service_entry("unhealthy-test", "dev", "http://unhealthy:8080", config.stale_threshold_ms + 500);
        let id = service.id.clone();
        reg.register(service).unwrap();
        id
    };

    // Verify health statuses
    {
        let reg = registry.read().await;
        let services = reg.list();
        
        let healthy_service = services.iter().find(|s| s.id == healthy_service_id).unwrap();
        assert!(matches!(healthy_service.health_status(&config), xolotl::model::service_registry::HealthStatus::Unknown)); // Just registered
        
        let stale_service = services.iter().find(|s| s.id == stale_service_id).unwrap();
        assert!(matches!(stale_service.health_status(&config), xolotl::model::service_registry::HealthStatus::Stale));
        
        let unhealthy_service = services.iter().find(|s| s.id == unhealthy_service_id).unwrap();
        assert!(matches!(unhealthy_service.health_status(&config), xolotl::model::service_registry::HealthStatus::Unhealthy));
    }

    // Cleanup should remove only the unhealthy service
    let removed_count = HealthManager::cleanup_unhealthy_services(&registry, &config)
        .await
        .unwrap();
    
    assert_eq!(removed_count, 1); // Only unhealthy service should be removed
    
    {
        let reg = registry.read().await;
        let services = reg.list();
        assert_eq!(services.len(), 2); // Healthy and stale should remain
        let remaining_services: Vec<&str> = services.iter().map(|s| s.service_name.as_str()).collect();
        assert!(remaining_services.contains(&"healthy-test"));
        assert!(remaining_services.contains(&"stale-test"));
        assert!(!remaining_services.contains(&"unhealthy-test"));
    }
}