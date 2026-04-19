use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post, patch},
    Router,
};
use sea_orm::{Database, ConnectOptions, DatabaseConnection};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

mod proxy;
mod api;
mod accounts;
mod entities;
mod epg;
mod auth;
mod m3u;
mod outputs;
mod xtream_codes;
mod channel_sync;

use axum::extract::State;

pub struct AppState {
    pub db: DatabaseConnection,
    pub http_client: reqwest::Client,
    pub ws_sender: tokio::sync::broadcast::Sender<serde_json::Value>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.ws_sender.subscribe();

    // Send connection_established to immediately satisfy the frontend
    let _ = socket.send(axum::extract::ws::Message::Text(
        serde_json::json!({
            "type": "connection_established",
            "data": {
                "message": "Connected to Rust backend"
            }
        }).to_string()
    )).await;

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

    let state = Arc::new(AppState {
        db,
        http_client: reqwest::Client::builder().build().unwrap(),
        ws_sender,
    });

    // SPA Routing: Serve index.html if the user hits a route like /channels directly
    let spa_service = ServeDir::new("dist")
        .not_found_service(ServeFile::new("dist/index.html"));

    let app = Router::new()
        // --- AUTH ---
        .route("/api/accounts/initialize-superuser/", get(api::check_superuser).post(accounts::init_superuser))
        .route("/api/accounts/users/me/", get(api::get_current_user).patch(accounts::update_me))
        .route("/api/accounts/users/", get(accounts::list_users).post(accounts::create_user))
        .route("/api/accounts/users/:id/", get(accounts::get_user).patch(accounts::update_user).delete(accounts::delete_user))
        .route("/api/accounts/groups/", get(accounts::list_groups).post(accounts::create_group))
        .route("/api/accounts/groups/:id/", get(accounts::get_group).patch(accounts::update_group).delete(accounts::delete_group))
        .route("/api/accounts/permissions/", get(accounts::list_permissions))
        .route("/api/accounts/api-keys/", get(accounts::get_api_key))
        .route("/api/accounts/api-keys/generate/", post(accounts::generate_api_key))
        .route("/api/accounts/api-keys/revoke/", post(accounts::revoke_api_key))
        .route("/api/accounts/token/", post(api::login))
        .route("/api/accounts/token/refresh/", post(api::refresh_token)) 
        .route("/api/accounts/auth/logout/", post(api::logout))

        // --- CORE & SETTINGS ---
        .route("/api/core/version/", get(api::get_core_version))
        .route("/api/core/settings/", get(api::get_core_settings))
        .route("/api/core/settings/env/", get(api::get_env_settings))
        .route("/api/core/timezones/", get(api::get_timezones))
        .route("/api/core/notifications/", get(api::get_notifications))
        .route("/api/core/useragents/", get(api::get_useragents))
        .route("/api/core/streamprofiles/", get(api::get_streamprofiles))

        // --- CHANNELS & M3U ---
        .route("/api/channels/channels/", get(api::get_channels))
        .route("/api/channels/groups/", get(api::get_channel_groups))
        .route("/api/channels/profiles/", get(api::get_channel_profiles))
        .route("/api/channels/channels/ids/", get(api::get_ids_stub))
        .route("/api/m3u/accounts/", get(api::get_m3u_accounts).post(api::add_m3u_account))
        .route("/api/m3u/accounts/:id/", get(api::get_m3u_account).patch(api::update_m3u_account).delete(api::delete_m3u_account))
        .route("/api/m3u/accounts/:id/group-settings/", patch(api::update_m3u_group_settings))
        .route("/api/m3u/accounts/:id/profiles/", get(api::get_m3u_profiles).post(api::create_m3u_profile))
        .route("/api/m3u/accounts/:id/profiles/:profile_id/", patch(api::update_m3u_profile).delete(api::delete_m3u_profile))
        .route("/api/m3u/accounts/:id/filters/", get(api::get_m3u_filters).post(api::create_m3u_filter))
        .route("/api/m3u/accounts/:id/filters/:filter_id/", patch(api::update_m3u_filter).delete(api::delete_m3u_filter))
        .route("/api/m3u/server-groups/", get(api::get_server_groups).post(api::create_server_group))
        .route("/api/m3u/server-groups/:group_id/", patch(api::update_server_group).delete(api::delete_server_group))
        .route("/api/m3u/refresh/", post(api::refresh_m3u_all))
        .route("/api/m3u/refresh/:id/", post(api::refresh_m3u_account))
        .route("/api/m3u/refresh-account-info/:profile_id/", post(api::refresh_m3u_account_info))
        
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
        .route("/api/channels/streams/", get(api::get_streams).post(api::post_stub))
        .route("/api/core/system-events/", get(api::get_paginated_object))
        .route("/api/connect/integrations/", get(api::get_paginated_object))
        .route("/api/plugins/plugins/", get(api::get_paginated_object))

        // --- OUTPUTS & PROVISIONING ---
        .route("/m3u/:token", get(outputs::generate_m3u))
        .route("/xmltv/:token", get(outputs::generate_xmltv))

        // --- SYSTEM & PROXY ---
        .route("/api/config/", get(api::get_config))
        .route("/ws/", get(ws_handler))
        .route("/play/:token/:channel_id", get(proxy::handle_proxy))
        
        // Serve the compiled React frontend for non-API routes
        .fallback_service(spa_service)
        .layer(CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("🚀 LISTENING ON {}", addr);
    axum::serve(listener, app).await.unwrap();
}