use axum::{routing::get, Router};
use sea_orm::{Database, ConnectOptions};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod proxy;
mod api;
mod entities;
mod epg;

pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in Unraid variables");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(20)
       .min_connections(5)
       .connect_timeout(Duration::from_secs(8))
       .idle_timeout(Duration::from_secs(8))
       .sqlx_logging(true);

    let db = Database::connect(opt)
        .await
        .expect("Failed to connect to Postgres.");

    let state = Arc::new(AppState {
        db: db.clone(),
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    });

    let app = Router::new()
        // API Routes with common aliases
        .route("/api/system/status", get(api::get_system_status))
        .route("/api/v1/system/status", get(api::get_system_status)) // Alias 1
        .route("/system/status", get(api::get_system_status))        // Alias 2
        
        .route("/api/config", get(api::get_config))
        .route("/api/v1/config", get(api::get_config))
        
        .route("/api/channels", get(api::get_channels))
        .route("/api/groups", get(api::get_groups))
        
        // Proxy
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        // UI Fallback (Must be last)
        .fallback_service(ServeDir::new("dist"))
        
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("🚀 DISPATCHARR-RS IS LIVE ON http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}