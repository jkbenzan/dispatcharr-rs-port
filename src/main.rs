use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use sea_orm::{Database, ConnectOptions, DatabaseConnection};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

mod proxy;
mod api;
mod entities;
mod epg;
mod auth;
mod m3u;
mod outputs;

pub struct AppState {
    pub db: DatabaseConnection,
    pub http_client: reqwest::Client,
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let axum::extract::ws::Message::Close(_) = msg { break; }
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 BACKEND STARTING...");
    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    let mut opt = ConnectOptions::new(db_url);
    opt.connect_timeout(Duration::from_secs(15));

    let db = Database::connect(opt).await.expect("DB Failure");
    println!("✅ DB CONNECTED");

    let state = Arc::new(AppState {
        db,
        http_client: reqwest::Client::builder().build().unwrap(),
    });

    // SPA Routing: Serve index.html if the user hits a route like /channels directly
    let spa_service = ServeDir::new("dist")
        .not_found_service(ServeFile::new("dist/index.html"));

    let app = Router::new()
        // --- AUTH ---
        .route("/api/accounts/initialize-superuser/", get(api::check_superuser))
        .route("/api/accounts/users/me/", get(api::get_current_user))
        .route("/api/accounts/token/", post(api::login))
        .route("/api/accounts/token/refresh/", post(api::refresh_token)) 
        .route("/api/accounts/auth/logout/", post(api::logout))

        // --- CORE & SETTINGS ---
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/settings/env/", get(api::get_env_settings))
        .route("/api/core/notifications/", get(api::get_notifications))
        .route("/api/core/useragents/", get(api::get_useragents))
        .route("/api/core/streamprofiles/", get(api::get_streamprofiles))

        // --- CHANNELS & M3U ---
        .route("/api/channels/channels/", get(api::get_channels))
        .route("/api/channels/groups/", get(api::get_channel_groups))
        .route("/api/channels/profiles/", get(api::get_profiles))
        .route("/api/channels/channels/ids/", get(api::get_ids_stub))
        .route("/api/m3u/accounts/", get(api::get_m3u_accounts))
        
        // --- EPG ---
        .route("/api/epg/sources/", get(api::get_epg_sources))
        .route("/api/epg/epgdata/", get(api::get_epgdata))

        // --- OUTPUTS & PROVISIONING ---
        .route("/m3u/:token", get(outputs::generate_m3u))
        .route("/xmltv/:token", get(outputs::generate_xmltv))

        // --- SYSTEM & PROXY ---
        .route("/api/config/", get(api::get_config))
        .route("/ws/", get(ws_handler))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        
        // Serve the compiled React frontend
        .fallback_service(spa_service)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("🚀 LISTENING ON {}", addr);
    axum::serve(listener, app).await.unwrap();
}