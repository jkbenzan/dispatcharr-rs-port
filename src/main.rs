use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Response, IntoResponse},
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

async fn logger_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let res = next.run(req).await;
    println!("📡 {} {} -> {}", method, uri, res.status());
    res
}

// Fixed: This handles the "Blank Screen on Refresh" by serving index.html for unknown routes
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
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let mut opt = ConnectOptions::new(db_url);
    opt.connect_timeout(Duration::from_secs(10));

    let db = Database::connect(opt).await.expect("DB Failure");
    println!("✅ DATABASE CONNECTED");

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
        .route("/api/accounts/auth/logout/", post(api::auth_placeholder))

        // Core
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/settings/env/", get(api::get_env_settings))
        .route("/api/core/notifications/", get(api::get_results_stub))
        .route("/api/core/streamprofiles/", get(api::get_results_stub))
        .route("/api/core/useragents/", get(api::get_results_stub))

        // Data
        .route("/api/channels/groups/", get(api::get_results_stub))
        .route("/api/channels/profiles/", get(api::get_results_stub))
        .route("/api/channels/channels/ids/", get(api::get_results_stub))
        .route("/api/m3u/accounts/", get(api::get_results_stub))
        .route("/api/epg/sources/", get(api::get_results_stub))
        .route("/api/epg/epgdata/", get(api::get_results_stub))

        .route("/api/config/", get(api::get_config))
        .route("/api/config", get(api::get_config))
        
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        // Static Files
        .nest_service("/assets", ServeDir::new("dist/assets"))
        
        // Fixed: The global fallback now points to our SPA handler
        .fallback(spa_fallback)
        
        .layer(middleware::from_fn(logger_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("🚀 RUNNING ON http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}