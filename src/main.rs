use api::services::services_routes;
use axum::Router;

mod api;
mod model;
mod registry;

#[tokio::main]
async fn main() {
    let app = create_app();
    let listener = match tokio::net::TcpListener::bind("0.0.0.0:8080").await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to address 0.0.0.0:8080: {}", e);
            std::process::exit(1);
        }
    };
    println!("Starting Xolotl on 0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    Router::new().nest("/services", services_routes())
}
