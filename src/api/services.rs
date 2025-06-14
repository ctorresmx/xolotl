use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::model::service_registry::{RegistryError, ServiceEntry, ServiceRegistry};

#[derive(Deserialize)]
struct ServiceEntryRequest {
    service_name: String,
    environment: String,
    address: String,
    tags: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct ServiceEntryResponse {
    service_name: String,
    environment: String,
    address: String,
    tags: HashMap<String, String>,
}

#[derive(Deserialize)]
struct HeartbeatRequest {
    service_name: String,
    environment: String,
}

pub fn services_routes() -> Router<Arc<RwLock<dyn ServiceRegistry>>> {
    Router::new()
        .route("/", get(list_services))
        .route("/", post(register_service))
        .route("/{name}/{environment}", get(get_service))
        .route(
            "/{name}/{environment}",
            delete(deregister_service_in_environment),
        )
        .route("/{name}", delete(deregister_service))
        .route("/heartbeat", put(register_heartbeat))
}

async fn register_heartbeat(
    State(registry): State<Arc<RwLock<dyn ServiceRegistry>>>,
    Json(payload): Json<HeartbeatRequest>,
) -> Result<Json<String>, StatusCode> {
    let mut registry = registry.write().await;
    let heartbeat_result = registry.heartbeat(&payload.service_name, &payload.environment);

    match heartbeat_result {
        Ok(_) => Ok(Json(format!(
            "Heartbeat received for service {} in {}",
            &payload.service_name, &payload.environment
        ))),
        Err(register_error) => match register_error {
            RegistryError::NotFound => Err(StatusCode::NOT_FOUND),
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
    }
}

async fn list_services(
    State(registry): State<Arc<RwLock<dyn ServiceRegistry>>>,
) -> Json<Vec<ServiceEntryResponse>> {
    let registry = registry.read().await;
    let services = registry
        .list()
        .iter()
        .map(|internal_entry| ServiceEntryResponse {
            service_name: internal_entry.service_name.clone(),
            environment: internal_entry.environment.clone(),
            address: internal_entry.address_str().to_string(),
            tags: internal_entry.tags.clone(),
        })
        .collect();
    Json(services)
}

async fn register_service(
    State(registry): State<Arc<RwLock<dyn ServiceRegistry>>>,
    Json(payload): Json<ServiceEntryRequest>,
) -> Result<Json<String>, StatusCode> {
    let mut registry = registry.write().await;
    let service_name = payload.service_name.clone();
    let service_environment = payload.environment.clone();
    let registering_result = registry.register(ServiceEntry::new(
        payload.service_name,
        payload.environment,
        payload.address,
        payload.tags.unwrap_or_default(),
    ));

    match registering_result {
        Ok(_) => Ok(Json(format!(
            "Successfully registered service {} in {}",
            service_name, service_environment,
        ))),
        Err(register_error) => match register_error {
            RegistryError::AlreadyExists => Err(StatusCode::CONFLICT),
            RegistryError::InternalError(msg) => {
                eprintln!("Internal error during registration: {}", msg);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
    }
}

async fn get_service(
    State(registry): State<Arc<RwLock<dyn ServiceRegistry>>>,
    Path((name, environment)): Path<(String, String)>,
) -> Result<Json<Vec<ServiceEntryResponse>>, StatusCode> {
    let registry = registry.read().await;
    let services = registry.resolve(&name, &environment);

    if services.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(
        services
            .iter()
            .map(|internal_entry| ServiceEntryResponse {
                service_name: internal_entry.service_name.clone(),
                environment: internal_entry.environment.clone(),
                address: internal_entry.address_str().to_string(),
                tags: internal_entry.tags.clone(),
            })
            .collect(),
    ))
}

async fn deregister_service(
    State(registry): State<Arc<RwLock<dyn ServiceRegistry>>>,
    Path(name): Path<String>,
) -> Result<Json<String>, StatusCode> {
    let mut registry = registry.write().await;

    let result = registry.deregister(&name, None);

    match result {
        Ok(_) => Ok(Json(format!("Successfully deregistered service {}", name))),
        Err(register_error) => match register_error {
            RegistryError::NotFound => Err(StatusCode::NOT_FOUND),
            RegistryError::InternalError(msg) => {
                eprintln!("Internal error during deregistration: {}", msg);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
    }
}

async fn deregister_service_in_environment(
    State(registry): State<Arc<RwLock<dyn ServiceRegistry>>>,
    Path((name, environment)): Path<(String, String)>,
) -> Result<Json<String>, StatusCode> {
    let mut registry = registry.write().await;

    let result = registry.deregister(&name, Some(&environment));

    match result {
        Ok(_) => Ok(Json(format!(
            "Successfully deregistered service {} in {}",
            name, environment
        ))),
        Err(register_error) => match register_error {
            RegistryError::NotFound => Err(StatusCode::NOT_FOUND),
            RegistryError::InternalError(msg) => {
                eprintln!("Internal error during deregistration: {}", msg);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
            _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::in_memory_registry::InMemoryRegistry;

    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use serde_json::{Value, json};
    use tokio::sync::RwLock;
    use tower::ServiceExt; // for `oneshot` and `ready`

    fn create_test_app() -> Router {
        let registry = Arc::new(RwLock::new(InMemoryRegistry::new()));
        services_routes().with_state(registry)
    }

    async fn send_request(app: Router, request: Request<Body>) -> (StatusCode, Value) {
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
        (status, json)
    }

    #[tokio::test]
    async fn test_register_service_success() {
        let app = create_test_app();

        let payload = json!({
            "service_name": "test-service",
            "environment": "dev",
            "address": "http://localhost:8080",
            "tags": {
                "version": "1.0.0",
                "team": "backend"
            }
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let (status, response) = send_request(app, request).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            response
                .as_str()
                .unwrap()
                .contains("Successfully registered service test-service in dev")
        );
    }

    #[tokio::test]
    async fn test_register_service_heartbeat() {
        let app = create_test_app();

        let payload = json!({
            "service_name": "test-service",
            "environment": "dev",
            "address": "http://localhost:8080",
            "tags": {
                "version": "1.0.0",
                "team": "backend"
            }
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let (status, response) = send_request(app.clone(), request).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            response
                .as_str()
                .unwrap()
                .contains("Successfully registered service test-service in dev")
        );

        let payload = json!({
            "service_name": "test-service",
            "environment": "dev",
        });

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/heartbeat")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let (status, response) = send_request(app, request).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            response
                .as_str()
                .unwrap()
                .contains("Heartbeat received for service test-service in dev")
        );
    }

    #[tokio::test]
    async fn test_register_service_minimal_payload() {
        let app = create_test_app();

        let payload = json!({
            "service_name": "minimal-service",
            "environment": "prod",
            "address": "http://api.example.com"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let (status, _) = send_request(app, request).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_register_service_invalid_json() {
        let app = create_test_app();

        let request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from("invalid json"))
            .unwrap();

        let (status, _) = send_request(app, request).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_services_empty() {
        let app = create_test_app();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app, request).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(response, json!([]));
    }

    #[tokio::test]
    async fn test_list_services_with_entries() {
        let app = create_test_app();

        // Register a service first
        let payload = json!({
            "service_name": "list-test",
            "environment": "dev",
            "address": "http://localhost:3000",
            "tags": { "type": "api" }
        });

        let register_request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        send_request(app.clone(), register_request).await;

        // Now list services
        let list_request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app, list_request).await;

        assert_eq!(status, StatusCode::OK);
        let services = response.as_array().unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0]["service_name"], "list-test");
        assert_eq!(services[0]["environment"], "dev");
        assert_eq!(services[0]["address"], "http://localhost:3000");
    }

    #[tokio::test]
    async fn test_get_service_found() {
        let app = create_test_app();

        // Register a service first
        let payload = json!({
            "service_name": "get-test",
            "environment": "staging",
            "address": "http://staging.example.com",
            "tags": { "version": "2.0.0" }
        });

        let register_request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        send_request(app.clone(), register_request).await;

        // Get the service
        let get_request = Request::builder()
            .method(Method::GET)
            .uri("/get-test/staging")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app, get_request).await;

        assert_eq!(status, StatusCode::OK);
        let services = response.as_array().unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services[0]["service_name"], "get-test");
        assert_eq!(services[0]["environment"], "staging");
    }

    #[tokio::test]
    async fn test_get_service_not_found() {
        let app = create_test_app();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/nonexistent/dev")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_deregister_service_success() {
        let app = create_test_app();

        // Register a service first
        let payload = json!({
            "service_name": "delete-test",
            "environment": "dev",
            "address": "http://localhost:4000"
        });

        let register_request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        send_request(app.clone(), register_request).await;

        // Delete the service
        let delete_request = Request::builder()
            .method(Method::DELETE)
            .uri("/delete-test")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app.clone(), delete_request).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            response
                .as_str()
                .unwrap()
                .contains("Successfully deregistered service delete-test")
        );

        // Verify it's gone
        let get_request = Request::builder()
            .method(Method::GET)
            .uri("/delete-test/dev")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, get_request).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_deregister_service_not_found() {
        let app = create_test_app();

        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/nonexistent")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_deregister_service_in_environment_success() {
        let app = create_test_app();

        // Register services in multiple environments
        let dev_payload = json!({
            "service_name": "multi-env-test",
            "environment": "dev",
            "address": "http://dev.example.com"
        });

        let prod_payload = json!({
            "service_name": "multi-env-test",
            "environment": "prod",
            "address": "http://prod.example.com"
        });

        for payload in [dev_payload, prod_payload] {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap();

            send_request(app.clone(), request).await;
        }

        // Delete only the dev environment
        let delete_request = Request::builder()
            .method(Method::DELETE)
            .uri("/multi-env-test/dev")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app.clone(), delete_request).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            response
                .as_str()
                .unwrap()
                .contains("Successfully deregistered service multi-env-test in dev")
        );

        // Verify dev is gone but prod remains
        let get_dev_request = Request::builder()
            .method(Method::GET)
            .uri("/multi-env-test/dev")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app.clone(), get_dev_request).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let get_prod_request = Request::builder()
            .method(Method::GET)
            .uri("/multi-env-test/prod")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, get_prod_request).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_deregister_service_in_environment_not_found() {
        let app = create_test_app();

        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/nonexistent/dev")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_multiple_service_registrations_same_name() {
        let app = create_test_app();

        let payload = json!({
            "service_name": "duplicate-test",
            "environment": "dev",
            "address": "http://localhost:5000"
        });

        // Register first time - should succeed
        let request1 = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let (status, _) = send_request(app.clone(), request1).await;
        assert_eq!(status, StatusCode::OK);

        // Register second time with same name/env but different address - should succeed
        // because services are identified by UUID, allowing multiple instances
        let payload2 = json!({
            "service_name": "duplicate-test",
            "environment": "dev",
            "address": "http://localhost:5001"
        });

        let request2 = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload2.to_string()))
            .unwrap();

        let (status, _) = send_request(app.clone(), request2).await;
        assert_eq!(status, StatusCode::OK);

        // Verify both instances exist
        let get_request = Request::builder()
            .method(Method::GET)
            .uri("/duplicate-test/dev")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app, get_request).await;
        assert_eq!(status, StatusCode::OK);

        let services = response.as_array().unwrap();
        assert_eq!(services.len(), 2);
    }

    #[tokio::test]
    async fn test_service_response_structure() {
        let app = create_test_app();

        // Register a service with all fields
        let payload = json!({
            "service_name": "structure-test",
            "environment": "test",
            "address": "https://api.test.com:443",
            "tags": {
                "version": "3.0.0",
                "team": "platform",
                "tier": "critical"
            }
        });

        let register_request = Request::builder()
            .method(Method::POST)
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        send_request(app.clone(), register_request).await;

        // Get and verify response structure
        let get_request = Request::builder()
            .method(Method::GET)
            .uri("/structure-test/test")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app, get_request).await;

        assert_eq!(status, StatusCode::OK);
        let services = response.as_array().unwrap();
        assert_eq!(services.len(), 1);

        let service = &services[0];
        assert_eq!(service["service_name"], "structure-test");
        assert_eq!(service["environment"], "test");
        assert_eq!(service["address"], "https://api.test.com:443");

        let tags = &service["tags"];
        assert_eq!(tags["version"], "3.0.0");
        assert_eq!(tags["team"], "platform");
        assert_eq!(tags["tier"], "critical");
    }

    #[tokio::test]
    async fn test_multiple_instances_same_service_environment() {
        let app = create_test_app();

        // Register first instance
        let payload1 = json!({
            "service_name": "load-balanced-service",
            "environment": "prod",
            "address": "http://instance1.example.com:8080",
            "tags": { "instance": "1" }
        });

        // Register second instance
        let payload2 = json!({
            "service_name": "load-balanced-service",
            "environment": "prod",
            "address": "http://instance2.example.com:8080",
            "tags": { "instance": "2" }
        });

        for payload in [payload1, payload2] {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap();

            let (status, _) = send_request(app.clone(), request).await;
            assert_eq!(status, StatusCode::OK);
        }

        // Get services - should return both instances
        let get_request = Request::builder()
            .method(Method::GET)
            .uri("/load-balanced-service/prod")
            .body(Body::empty())
            .unwrap();

        let (status, response) = send_request(app, get_request).await;

        assert_eq!(status, StatusCode::OK);
        let services = response.as_array().unwrap();
        assert_eq!(services.len(), 2);

        let addresses: Vec<&str> = services
            .iter()
            .map(|s| s["address"].as_str().unwrap())
            .collect();

        assert!(addresses.contains(&"http://instance1.example.com:8080"));
        assert!(addresses.contains(&"http://instance2.example.com:8080"));
    }
}
