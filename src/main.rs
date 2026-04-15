use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
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

async fn logger_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let res = next.run(req).await;
    
    // Log the request AND the result (so we see the 404s)
    println!("📡 {} {} -> {}", method, uri, res.status());
    res
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(10)
       .connect_timeout(Duration::from_secs(10)) 
       .sqlx_logging(true);

    let db = Database::connect(opt).await.expect("DB Failure");
    println!("✅ DATABASE CONNECTED");

    let state = Arc::new(AppState {
        db,
        http_client: reqwest::Client::builder().build().unwrap(),
    });

    let app = Router::new()
        // API routes
        .route("/api/system/status", get(api::get_system_status))
        .route("/api/v1/system/status", get(api::get_system_status))
        .route("/api/config", get(api::get_config))
        .route("/api/channels", get(api::get_channels))
        .route("/api/groups", get(api::get_groups))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        // Static files (The UI)
        .fallback_service(
            ServeDir::new("dist").append_index_html_on_directories(true)
        )
        
        .layer(middleware::from_fn(logger_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("🚀 RUNNING ON http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}