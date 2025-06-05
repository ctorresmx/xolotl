use api::services::services_routes;
use axum::Router;
use clap::Parser;

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
    axum::serve(listener, app).await.unwrap();
}

fn create_app() -> Router {
    Router::new().nest("/services", services_routes())
}
