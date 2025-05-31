use api::services::services_routes;
use axum::Router;

mod api;

#[tokio::main]
async fn main() {
    let app = create_app();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Starting Xolotl on 0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    Router::new().nest("/services", services_routes())
}
