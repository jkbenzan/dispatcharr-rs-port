use axum::{routing::get, Router, http::Request, body::Body};
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
    // 1. Initialize logging with immediate flush
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in your Unraid Docker template");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    // 2. Database Connection with Strict Timeouts
    let mut opt = ConnectOptions::new(db_url.clone());
    opt.max_connections(10)
       .connect_timeout(Duration::from_secs(10)) // If it can't connect in 10s, fail
       .acquire_timeout(Duration::from_secs(10))
       .sqlx_logging(true);

    println!("--------------------------------------------------");
    println!("🔍 DEBUG: Attempting to connect to: {}", db_url); 
    println!("--------------------------------------------------");

    // We use a match here so the app doesn't just "disappear" on failure
    let db = match Database::connect(opt).await {
        Ok(conn) => {
            println!("✅ DATABASE CONNECTED SUCCESSFULLY");
            conn
        },
        Err(e) => {
            println!("❌ DATABASE ERROR: {:?}", e);
            panic!("Could not connect to database. Check your DATABASE_URL in Unraid.");
        }
    };

    let state = Arc::new(AppState {
        db,
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap(),
    });

    let app = Router::new()
        .route("/api/system/status", get(api::get_system_status))
        .route("/api/v1/system/status", get(api::get_system_status))
        .route("/api/config", get(api::get_config))
        .route("/api/channels", get(api::get_channels))
        .route("/api/groups", get(api::get_groups))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        .fallback_service(
            ServeDir::new("dist").append_index_html_on_directories(true)
        )
        
        .layer(axum::middleware::map_request(|req: Request<Body>| {
            println!("📥 REQUEST: {} {}", req.method(), req.uri());
            req
        }))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    println!("🚀 SERVER STARTING ON http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}