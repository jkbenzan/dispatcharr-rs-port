use axum::{routing::get, Router, extract::State};
use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

mod proxy;
mod api;
mod entities;

pub struct AppState {
    pub db: DatabaseConnection,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").unwrap_or("sqlite://dispatcharr.db?mode=rwc".into());
    
    let db = Database::connect(db_url).await.expect("Failed to connect to DB");
    let state = Arc::new(AppState {
        db,
        http_client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        .route("/api/channels", get(api::get_channels))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("🚀 Dispatcharr Rust Core running on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}
