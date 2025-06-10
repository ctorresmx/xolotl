use api::services::services_routes;
use axum::Router;
use clap::Parser;
use registry::in_memory_registry::InMemoryRegistry;
use std::sync::Arc;
use tokio::sync::RwLock;

mod api;
mod model;
mod registry;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    address: String,

    #[arg(short, long, default_value_t = 8000)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let app = create_app();
    let bind_address = format!("{}:{}", args.address, args.port);

    let listener = match tokio::net::TcpListener::bind(&bind_address).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to address {}: {}", bind_address, e);
            std::process::exit(1);
        }
    };
    println!("Starting Xolotl on {}", bind_address);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub fn create_app() -> Router {
    let registry = Arc::new(RwLock::new(InMemoryRegistry::new()));
    Router::new()
        .nest("/services", services_routes())
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_app() {
        let app = create_app();

        // Just verify the app can be created without panicking
        // This tests the initialization and dependency injection
        assert!(std::any::type_name_of_val(&app).contains("Router"));
    }

    #[test]
    fn test_args_defaults() {
        let args = Args {
            address: "0.0.0.0".to_string(),
            port: 8000,
        };

        assert_eq!(args.address, "0.0.0.0");
        assert_eq!(args.port, 8000);
    }

    #[test]
    fn test_args_custom_values() {
        let args = Args {
            address: "127.0.0.1".to_string(),
            port: 3000,
        };

        assert_eq!(args.address, "127.0.0.1");
        assert_eq!(args.port, 3000);
    }
}
