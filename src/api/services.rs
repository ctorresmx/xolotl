use axum::{
    Json, Router,
    extract::Path,
    routing::{delete, get, post},
};

pub fn services_routes() -> Router {
    Router::new()
        .route("/", get(list_services))
        .route("/", post(register_service))
        .route("/{name}/{env}", get(get_service))
        .route("/{name}", delete(deregister_service))
}

async fn list_services() -> Json<String> {
    Json(format!("Listing services"))
}

async fn register_service() -> Json<String> {
    Json(format!("Registering service"))
}

async fn get_service(Path((name, env)): Path<(String, String)>) -> Json<String> {
    Json(format!("Retrieving {} in {}", name, env))
}

async fn deregister_service(Path(name): Path<String>) -> Json<String> {
    Json(format!("Deregistering service {}", name))
}
