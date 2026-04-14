use axum::{routing::get, Router};
use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;

mod proxy;
mod api;
mod entities;
mod epg; // Assuming you'll add epg parsing logic here

pub struct AppState {
    pub db: DatabaseConnection,
    pub http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // 1. Load Environment Variables
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    // 2. Configure Database Connection (Postgres Optimized)
    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(20)
       .min_connections(5)
       .connect_timeout(Duration::from_secs(8))
       .idle_timeout(Duration::from_secs(8))
       .max_lifetime(Duration::from_secs(8))
       .sqlx_logging(true);

    let db = Database::connect(opt)
        .await
        .expect("Failed to connect to the database. Check your DATABASE_URL.");

    let state = Arc::new(AppState {
        db: db.clone(),
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    });

    // 3. Spawn Background Tasks (EPG Refresh)
    let epg_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600 * 12)); // Every 12 hours
        loop {
            interval.tick().await;
            println!("🔄 Starting background EPG refresh...");
             epg::refresh_all_guides(&epg_state.db).await; 
        }
    });

    // 4. Define API Routes & Middleware
    let app = Router::new()
        // Stream Proxying (Matches legacy Dispatcharr URL structure)
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        
        // Frontend API Parity
        .route("/api/channels", get(api::get_channels))
        .route("/api/groups", get(api::get_groups)) // Placeholder for group logic
        
        // Layers
        .layer(CorsLayer::permissive())
        .with_state(state);

    // 5. Start the Server
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("--------------------------------------------------");
    println!("🚀 DISPATCHARR-RS IS LIVE");
    println!("📡 Listening on: http://{}", addr);
    println!("🗄️  Database: Connected via PostgreSQL");
    println!("--------------------------------------------------");

    axum::serve(listener, app).await.unwrap();
}