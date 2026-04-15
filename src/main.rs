use axum::{
    body::Body,
    extract::ws::WebSocketUpgrade,
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

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        while let Some(Ok(_msg)) = socket.recv().await {}
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
    println!("🚀 DISPATCHARR-RS IS STARTING UP...");

    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    
    let mut opt = ConnectOptions::new(db_url.clone());
    opt.connect_timeout(Duration::from_secs(15));

    let mut db_conn = None;
    for i in 1..=5 {
        println!("🔄 DB CONNECTION ATTEMPT {}/5...", i);
        match Database::connect(opt.clone()).await {
            Ok(conn) => {
                println!("✅ DATABASE CONNECTED");
                db_conn = Some(conn);
                break;
            }
            Err(e) => {
                println!("⚠️  ATTEMPT {} FAILED: {:?}", i, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    let state = Arc::new(AppState {
        db: db_conn.expect("DB Failure"),
        http_client: reqwest::Client::builder().build().unwrap(),
    });

    let app = Router::new()
        .route("/api/accounts/initialize-superuser/", get(api::check_superuser))
        .route("/api/accounts/users/me/", get(api::get_current_user))
        .route("/api/accounts/token/", post(api::auth_placeholder))
        .route("/api/accounts/token/refresh/", post(api::auth_placeholder))
        .route("/api/accounts/auth/logout/", post(api::logout_stub))
        
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/notifications/", get(api::get_notifications))
        .route("/api/core/useragents/", get(api::get_flat_list))
        .route("/api/core/streamprofiles/", get(api::get_flat_list))
        
        .route("/api/channels/groups/", get(api::get_flat_list))
        .route("/api/channels/profiles/", get(api::get_flat_list))
        .route("/api/m3u/accounts/", get(api::get_flat_list))
        .route("/api/epg/sources/", get(api::get_flat_list))
        .route("/api/epg/epgdata/", get(api::get_flat_list))
        
        .route("/api/config/", get(api::get_config))
        .route("/ws/", get(ws_handler))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        
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