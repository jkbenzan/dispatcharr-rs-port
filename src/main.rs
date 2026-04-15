use axum::{routing::get, Router};
use sea_orm::{Database, ConnectOptions};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer; // Added for detailed request logging

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
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in your Unraid Docker template");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    // 2. Database Connection
    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(20)
       .connect_timeout(Duration::from_secs(10))
       .sqlx_logging(true);

    let db = Database::connect(opt)
        .await
        .expect("CRITICAL: Database connection failed.");

    let state = Arc::new(AppState {
        db: db.clone(),
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    });

    // 3. Routing
    let app = Router::new()
        // SYSTEM & CONFIG (Covering all known Dispatcharr variations)
        .route("/api/system/status", get(api::get_system_status))
        .route("/api/v1/system/status", get(api::get_system_status))
        .route("/system/status", get(api::get_system_status))
        .route("/api/config", get(api::get_config))
        .route("/api/v1/config", get(api::get_config))
        
        // DATA ENDPOINTS
        .route("/api/channels", get(api::get_channels))
        .route("/api/v1/channels", get(api::get_channels))
        .route("/api/groups", get(api::get_groups))
        .route("/api/v1/groups", get(api::get_groups))
        
        // PROXY
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        // UI SERVING
        .fallback_service(
            ServeDir::new("dist").append_index_html_on_directories(true)
        )
        
        // LAYERS
        .layer(TraceLayer::new_for_http()) // This will log every URL the browser hits
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("--------------------------------------------------");
    println!("🚀 DISPATCHARR-RS IS LIVE");
    println!("📡 URL: http://{}", addr);
    println!("--------------------------------------------------");

    axum::serve(listener, app).await.unwrap();
}