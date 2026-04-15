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
    // 1. Environment & Config
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in Unraid variables");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    // 2. Database Connection (Postgres Optimized)
    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(20)
       .min_connections(5)
       .connect_timeout(Duration::from_secs(8))
       .idle_timeout(Duration::from_secs(8))
       .sqlx_logging(true);

    let db = Database::connect(opt)
        .await
        .expect("Failed to connect to Postgres. Check your DATABASE_URL.");

    let state = Arc::new(AppState {
        db: db.clone(),
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    });

    // 3. Background Tasks (EPG)
    let epg_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600 * 12));
        loop {
            interval.tick().await;
            epg::refresh_all_guides(&epg_state.db).await; 
        }
    });

    // 4. Routes (Order Matters!)
    let app = Router::new()
        // API Routes - These must stay at the top
        .route("/api/system/status", get(api::get_system_status))
        .route("/api/channels", get(api::get_channels))
        .route("/api/groups", get(api::get_groups))
        
        // Proxy Routes
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        // Static File Serving - This acts as the "Fallback"
        // It serves files from the /app/dist folder inside the container
        .fallback_service(ServeDir::new("dist"))
        
        // Layers & Shared State
        .layer(CorsLayer::permissive())
        .with_state(state);

    // 5. Start Server
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("--------------------------------------------------");
    println!("🚀 DISPATCHARR-RS IS LIVE");
    println!("📡 Listening on: http://{}", addr);
    println!("🗄️  Database: PostgreSQL Connected");
    println!("📂 UI: Serving from ./dist");
    println!("--------------------------------------------------");

    axum::serve(listener, app).await.unwrap();
}