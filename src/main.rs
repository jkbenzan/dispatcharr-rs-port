use axum::{
    body::Body,
    http::Request,
    middleware::{self, Next},
    response::Response,
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
        // --- AUTH & ACCOUNTS ---
        .route("/api/accounts/initialize-superuser/", get(api::check_superuser))
        .route("/api/accounts/users/me/", get(api::get_current_user))
        .route("/api/accounts/token/", post(api::auth_placeholder))
        .route("/api/accounts/token/refresh/", post(api::refresh_token)) // FIXED: Added refresh route
        .route("/api/accounts/auth/logout/", post(api::auth_placeholder))

        // --- CORE & SETTINGS ---
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/settings/env/", get(api::get_env_settings))
        .route("/api/core/notifications/", get(api::get_notifications))
        .route("/api/core/streamprofiles/", get(api::get_profiles))
        .route("/api/core/useragents/", get(api::get_profiles))

        // --- CHANNELS, M3U & EPG ---
        .route("/api/channels/groups/", get(api::get_channel_groups))
        .route("/api/channels/profiles/", get(api::get_profiles))
        .route("/api/channels/channels/ids/", get(api::get_ids_stub))
        .route("/api/m3u/accounts/", get(api::get_m3u_accounts))
        .route("/api/epg/sources/", get(api::get_epg_sources))
        .route("/api/epg/epgdata/", get(api::get_epg_sources))

        .route("/api/config/", get(api::get_config))
        .route("/api/config", get(api::get_config))
        
        // --- PROXY ---
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))

        // --- UI ---
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