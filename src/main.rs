use axum::{
    body::Body,
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
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

// Dummy WebSocket handler to stop console errors
async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        while let Some(Ok(_msg)) = socket.recv().await {
            // Just keep the connection alive
        }
    })
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
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    
    let mut opt = ConnectOptions::new(db_url);
    opt.connect_timeout(Duration::from_secs(10));
    let db = Database::connect(opt).await.expect("DB Failure");

    let state = Arc::new(AppState {
        db,
        http_client: reqwest::Client::builder().build().unwrap(),
    });

    let app = Router::new()
        // Auth
        .route("/api/accounts/initialize-superuser/", get(api::check_superuser))
        .route("/api/accounts/users/me/", get(api::get_current_user))
        .route("/api/accounts/token/", post(api::auth_placeholder))
        .route("/api/accounts/token/refresh/", post(api::auth_placeholder))

        // System & Data (Switching back to flat lists to fix .reduce() errors)
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/notifications/", get(api::get_flat_list))
        
        .route("/api/channels/groups/", get(api::get_flat_list))
        .route("/api/channels/profiles/", get(api::get_flat_list))
        .route("/api/m3u/accounts/", get(api::get_flat_list))
        .route("/api/epg/sources/", get(api::get_flat_list))
        .route("/api/epg/epgdata/", get(api::get_flat_list))

        // WebSocket
        .route("/ws/", get(ws_handler))

        .fallback_service(ServeDir::new("dist").not_found_service(get(spa_fallback)))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}