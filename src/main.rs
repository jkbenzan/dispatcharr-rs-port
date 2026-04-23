use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, patch, post},
    Router,
};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

mod accounts;
mod api;
mod stream_checker;
mod auth;
mod channel_sync;
mod entities;
mod epg;
mod m3u;
mod outputs;
mod proxy;
mod settings;
mod vod;
mod xtream_codes;

use axum::extract::State;

pub struct AppState {
    pub db: DatabaseConnection,
    pub http_client: reqwest::Client,
    pub ws_sender: tokio::sync::broadcast::Sender<serde_json::Value>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.ws_sender.subscribe();

    // Send connection_established to immediately satisfy the frontend
    let _ = socket
        .send(axum::extract::ws::Message::Text(
            serde_json::json!({
                "type": "connection_established",
                "data": {
                    "message": "Connected to Rust backend"
                }
            })
            .to_string(),
        ))
        .await;

    // Loop to handle incoming pings/close and outgoing broadcast messages
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(payload) => {
                        let wrapped = serde_json::json!({ "data": payload });
                        if socket.send(axum::extract::ws::Message::Text(wrapped.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(msg)) => {
                        if let axum::extract::ws::Message::Close(_) = msg {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tracing::info!("🚀 BACKEND STARTING...");
    dotenvy::dotenv().ok();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL missing");
    let mut opt = ConnectOptions::new(db_url);
    opt.connect_timeout(Duration::from_secs(15))
        .sqlx_logging(false);

    let db = Database::connect(opt).await.expect("DB Failure");
    println!("✅ DB CONNECTED");

    // Create a broadcast channel for websockets with a capacity of 100
    let (ws_sender, _) = tokio::sync::broadcast::channel(100);

    let http_client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();

    let state = Arc::new(AppState {
        db,
        http_client,
        ws_sender,
    });

    // SPA Routing: Serve index.html if the user hits a route like /channels directly
    let spa_service = ServeDir::new("dist").fallback(ServeFile::new("dist/index.html"));

    let accounts_routes = Router::new()
        .route(
            "/users/",
            get(accounts::list_users).post(accounts::create_user),
        )
        .route(
            "/users/me/",
            get(accounts::get_me).patch(accounts::update_me),
        )
        .route(
            "/users/:id/",
            get(accounts::get_user)
                .put(accounts::update_user)
                .patch(accounts::update_user)
                .delete(accounts::delete_user),
        )
        .route(
            "/groups/",
            get(accounts::list_groups).post(accounts::create_group),
        )
        .route(
            "/groups/:id/",
            get(accounts::get_group)
                .put(accounts::update_group)
                .patch(accounts::update_group)
                .delete(accounts::delete_group),
        )
        .route("/permissions/", get(accounts::list_permissions))
        .route("/api-keys/", get(accounts::get_api_key))
        .route("/api-keys/generate/", post(accounts::generate_api_key))
        .route("/api-keys/revoke/", post(accounts::revoke_api_key))
        .route(
            "/initialize-superuser/",
            get(accounts::check_superuser).post(accounts::init_superuser),
        );

    // Moved settings routes directly to main router to avoid nest trailing slash issues

    let vod_routes = Router::new()
        .route("/all/", get(vod::get_vod_all))
        .route("/categories/", get(vod::get_vod_categories))
        .route("/movies/", get(vod::get_vod_movies))
        .route("/series/", get(vod::get_vod_series));

    let app = Router::new()
        // --- AUTH ---
        .route("/api/accounts/token/", post(api::login))
        .route("/api/accounts/token/refresh/", post(api::refresh_token))
        .route("/api/accounts/auth/logout/", post(api::logout))
        .route("/api/accounts/auth/login/", post(api::login))
        // --- CORE & SETTINGS ---
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/timezones/", get(api::get_timezones))
        .route("/api/core/notifications/", get(api::get_notifications))
        .route("/api/core/notifications/count/", get(api::get_notifications_count))
        .route("/api/core/useragents/", get(api::get_useragents))
        // Explicitly map settings routes to handle trailing slashes robustly
        .route(
            "/api/core/settings",
            get(settings::list_settings).post(settings::create_setting),
        )
        .route(
            "/api/core/settings/",
            get(settings::list_settings).post(settings::create_setting),
        )
        .route("/api/core/settings/env/", get(api::get_env_settings))
        .route(
            "/api/core/settings/:id/",
            get(settings::get_setting)
                .put(settings::update_setting)
                .patch(settings::update_setting),
        )
        .route("/api/core/settings/:id/check/", get(settings::get_setting))
        .route("/api/core/streamprofiles/", get(api::get_streamprofiles))
        // --- CHANNELS & M3U ---
        .route("/api/channels/channels/", get(api::get_channels))
        .route("/api/channels/groups/", get(api::get_channel_groups))
        .route("/api/channels/profiles/", get(api::get_channel_profiles))
        .route("/api/channels/channels/ids/", get(api::get_ids_stub))
        .route(
            "/api/channels/dvr/comskip-config/",
            get(api::get_comskip_config).post(api::upload_comskip_ini),
        )
        .route(
            "/api/m3u/accounts/",
            get(api::get_m3u_accounts).post(api::add_m3u_account),
        )
        .route(
            "/api/m3u/accounts/:id/",
            get(api::get_m3u_account)
                .patch(api::update_m3u_account)
                .delete(api::delete_m3u_account),
        )
        .route(
            "/api/m3u/accounts/:id/group-settings/",
            patch(api::update_m3u_group_settings),
        )
        .route("/api/m3u/accounts/:id/refresh-vod/", post(api::refresh_vod))
        .route(
            "/api/m3u/accounts/:id/profiles/",
            get(api::get_m3u_profiles).post(api::create_m3u_profile),
        )
        .route(
            "/api/m3u/accounts/:id/profiles/:profile_id/",
            patch(api::update_m3u_profile).delete(api::delete_m3u_profile),
        )
        .route(
            "/api/m3u/accounts/:id/filters/",
            get(api::get_m3u_filters).post(api::create_m3u_filter),
        )
        .route(
            "/api/m3u/accounts/:id/filters/:filter_id/",
            patch(api::update_m3u_filter).delete(api::delete_m3u_filter),
        )
        .route(
            "/api/m3u/server-groups/",
            get(api::get_server_groups).post(api::create_server_group),
        )
        .route(
            "/api/m3u/server-groups/:group_id/",
            patch(api::update_server_group).delete(api::delete_server_group),
        )
        .route("/api/m3u/refresh/", post(api::refresh_m3u_all))
        .route("/api/m3u/refresh/:id/", post(api::refresh_m3u_account))
        .route(
            "/api/m3u/refresh-account-info/:profile_id/",
            post(api::refresh_m3u_account_info),
        )
        // --- EPG ---
        .route("/api/epg/sources/", get(api::get_epg_sources))
        .route("/api/epg/sources/:id/", get(api::get_epg_source))
        .route("/api/epg/refresh/:id/", post(api::refresh_epg_source))
        .route("/api/epg/epgdata/", get(api::get_epgdata))
        // --- DASHBOARD MISSING DEPENDENCIES ---
        .route("/api/channels/logos/", get(api::get_flat_array))
        .route("/api/channels/streams/ids/", get(api::get_flat_array))
        .route("/api/channels/streams/filter-options/", get(api::get_stream_filter_options))
        .route("/api/channels/dashboard-stats/", get(api::get_dashboard_stats))
        .route("/api/channels/streams/", get(api::get_streams).post(api::create_stream))
        .route("/api/channels/streams/:id/test/", post(stream_checker::checker::test_stream))
        .route("/api/core/system-events/", get(api::get_paginated_object))
        .route("/api/connect/integrations/", get(api::get_paginated_object))
        .route("/api/plugins/plugins/", get(api::get_paginated_object))
        // --- OUTPUTS & PROVISIONING ---
        .route("/m3u/:token", get(outputs::generate_m3u))
        .route("/xmltv/:token", get(outputs::generate_xmltv))
        // --- SYSTEM & PROXY ---
        .route("/api/config/", get(api::get_config))
        .route("/ws", get(ws_handler))
        .route("/ws/", get(ws_handler))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        // Serve the compiled React frontend for non-API routes
        .nest("/api/accounts", accounts_routes)
        .nest("/api/vod", vod_routes)
        .fallback_service(spa_service)
        .layer(CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state.clone());

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("🚀 LISTENING ON {}", addr);
    println!(
        "🚀 Rust Dispatcharr API listening on {}",
        listener.local_addr().unwrap()
    );

    // Spawn a secondary listener on port 8001 specifically for WebSockets
    // This provides backward compatibility with the old Django/Daphne Nginx configuration.
    let state_clone = state.clone();
    tokio::spawn(async move {
        let ws_app = Router::new()
            .route("/ws", get(ws_handler))
            .route("/ws/", get(ws_handler))
            .with_state(state_clone)
            .layer(CorsLayer::permissive());

        if let Ok(ws_listener) = tokio::net::TcpListener::bind("0.0.0.0:8001").await {
            println!(
                "📡 WebSocket Compatibility Server listening on {}",
                ws_listener.local_addr().unwrap()
            );
            let _ = axum::serve(ws_listener, ws_app).await;
        }
    });

    axum::serve(listener, app).await.unwrap();
}
