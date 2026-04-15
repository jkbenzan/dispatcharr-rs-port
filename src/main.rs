use axum::{
    body::Body,
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use sea_orm::{Database, ConnectOptions, DatabaseConnection};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

mod proxy;
mod api;
mod entities;
mod epg;

pub struct AppState {
    pub db: DatabaseConnection,
    pub http_client: reqwest::Client,
}

// Improved WebSocket handler to ensure the connection stays alive
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let axum::extract::ws::Message::Close(_) = msg {
            break;
        }
    }
}

async fn spa_fallback() -> impl IntoResponse {
    let index_content = tokio::fs::read_to_string("dist/index.html")
        .await
        .unwrap_or_else(|_| "index.html not found".to_string());
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(Body::from(index_content))
        .unwrap()
}

#[tokio::main]
async fn main() {
    println!("🚀 DISPATCHARR-RS IS STARTING UP...");
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    
    let mut opt = ConnectOptions::new(db_url.clone());
    opt.connect_timeout(Duration::from_secs(15));

    let mut db_conn = None;
    for i in 1..=5 {
        if let Ok(conn) = Database::connect(opt.clone()).await {
            db_conn = Some(conn);
            break;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    let state = Arc::new(AppState {
        db: db_conn.expect("DB Failure"),
        http_client: reqwest::Client::builder().build().unwrap(),
    });

    let app = Router::new()
        // Auth
        .route("/api/accounts/initialize-superuser/", get(api::check_superuser))
        .route("/api/accounts/users/me/", get(api::get_current_user))
        .route("/api/accounts/token/", post(api::auth_placeholder))
        .route("/api/accounts/token/refresh/", post(api::auth_placeholder))
        .route("/api/accounts/auth/logout/", post(api::logout_stub))
        
        // Settings & Core
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/settings/env/", get(api::get_env_settings))
        .route("/api/core/notifications/", get(api::get_results_stub))
        .route("/api/core/useragents/", get(api::get_results_stub))
        .route("/api/core/streamprofiles/", get(api::get_results_stub))
        
        // Data Channels (Added missing epgdata route)
        .route("/api/channels/groups/", get(api::get_results_stub))
        .route("/api/channels/profiles/", get(api::get_results_stub))
        .route("/api/channels/channels/ids/", get(api::get_results_stub))
        .route("/api/m3u/accounts/", get(api::get_results_stub))
        .route("/api/epg/sources/", get(api::get_results_stub))
        .route("/api/epg/epgdata/", get(api::get_results_stub))
        
        .route("/api/config/", get(api::get_config))
        .route("/ws/", get(ws_handler))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        
        // SPA & Static Files
        .fallback_service(
            ServeDir::new("dist").not_found_service(get(spa_fallback))
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    println!("🚀 LISTENING ON http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}