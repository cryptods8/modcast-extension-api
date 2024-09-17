use axum::Router;
use dotenvy::dotenv;
use std::env;

mod routes;
mod airstack;
mod cache;

#[tokio::main]
async fn main() {
    dotenv().ok();
    // build our application with a single route
    let app = Router::new().nest("/api/v1", routes::api_routes());

    let port = env::var("PORT").unwrap_or("4000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Listening on {}", addr);
    
    axum::serve(listener, app).await.unwrap();
}
