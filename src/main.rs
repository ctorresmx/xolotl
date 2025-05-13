use axum::{Json, Router, routing::get};

#[tokio::main]
async fn main() {
    let router = Router::new().route("/services", get(list_services));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn list_services() -> Json<Vec<String>> {
    Json(vec!["Hello, World".to_string()])
}
