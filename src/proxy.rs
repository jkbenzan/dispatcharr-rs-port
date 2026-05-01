use crate::{
    entities::{channel, channel_stream, stream, user, core_settings},
    AppState,
    auth::verify_password,
};
use axum::{
    body::Body,
    extract::{Path, State, Query, ConnectInfo},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Serialize, Clone, Debug)]
pub struct ClientStat {
    pub client_id: String,
    pub user_id: Option<i64>,
    pub ip: String,
    pub user_agent: String,
    pub connected_at: u64,
}

#[derive(Clone, Debug)]
pub struct ChannelStats {
    pub channel_id: String,
    pub stream_id: String,
    pub stream_profile: String,
    pub provider_user_agent: String,
    pub uptime: u64,
    pub total_bytes: Arc<AtomicU64>,
    pub clients: Vec<ClientStat>,
    pub start_time: u64,
}

struct ClientDropGuard {
    channel_id: String,
    client_id: String,
    state: Arc<AppState>,
}

impl Drop for ClientDropGuard {
    fn drop(&mut self) {
        let channel_id = self.channel_id.clone();
        let client_id = self.client_id.clone();
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut active = state.active_streams.write().await;
            if let Some(stats) = active.get_mut(&channel_id) {
                stats.clients.retain(|c| c.client_id != client_id);
                if stats.clients.is_empty() {
                    active.remove(&channel_id);
                }
            }
        });
    }
}

#[derive(Deserialize)]
pub struct ProxyQuery {
    pub u: Option<String>,
    pub p: Option<String>,
    pub token: Option<String>,
}

async fn get_stream_fallback(
    channel_id: &str,
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<(channel_stream::Model, Option<stream::Model>)>, StatusCode> {
    let _stream = stream::Entity::find()
        .filter(stream::Column::StreamHash.eq(channel_id))
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(vec![(
        channel_stream::Model {
            id: 0,
            channel_id: 0,
            stream_id: _stream.id,
            order: 0,
        },
        Some(_stream),
    )])
}

pub async fn handle_ts_status(State(state): State<Arc<AppState>>) -> axum::Json<serde_json::Value> {
    let active = state.active_streams.read().await;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut channels = Vec::new();
    for (_, stat) in active.iter() {
        channels.push(serde_json::json!({
            "channel_id": stat.channel_id,
            "stream_id": stat.stream_id,
            "stream_profile": stat.stream_profile,
            "provider_user_agent": stat.provider_user_agent,
            "uptime": now.saturating_sub(stat.start_time),
            "total_bytes": stat.total_bytes.load(Ordering::Relaxed),
            "clients": stat.clients,
        }));
    }

    axum::Json(serde_json::json!({
        "channels": channels
    }))
}

pub async fn handle_vod_stats() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!([]))
}

pub async fn handle_proxy(
    Path(channel_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProxyQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
) -> Result<Response<Body>, StatusCode> {
    let client_user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    // Enhanced IP Detection (check X-Forwarded-For for Nginx/Docker)
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    // 1. Identify User
    let mut authenticated_user: Option<user::Model> = None;
    if let (Some(u), Some(p)) = (query.u, query.p) {
        let user_opt = user::Entity::find()
            .filter(user::Column::Username.eq(u))
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        if let Some(user) = user_opt {
            if verify_password(&user.password, &p) {
                authenticated_user = Some(user);
            }
        }
    } else if let Some(token) = query.token {
        // Simple token auth if implemented, for now skip
    }

    // If no auth provided, for now we might allow it but ideally we should require it
    // Original Django app might have a default user or based on setting.
    // For now, if no auth, we'll assign to "None" and skip limits if so configured.

    // 2. Enforce User Limits
    if let Some(user) = &authenticated_user {
        let limit = user.stream_limit;
        if limit > 0 {
            // Count active streams for this user
            let mut user_clients = Vec::new(); // (channel_id, client_id, connected_at)
            {
                let active = state.active_streams.read().await;
                for (ch_id, stats) in active.iter() {
                    for client in &stats.clients {
                        if client.user_id == Some(user.id) {
                            user_clients.push((ch_id.clone(), client.client_id.clone(), client.connected_at));
                        }
                    }
                }
            }

            if user_clients.len() >= limit as usize {
                // Fetch limit policy
                let settings = core_settings::Entity::find()
                    .filter(core_settings::Column::Key.eq("user_limit_settings"))
                    .one(&state.db)
                    .await
                    .unwrap_or_default();
                
                let mut terminate_oldest = true;
                if let Some(s) = settings {
                    terminate_oldest = s.value.get("terminate_oldest").and_then(|v| v.as_bool()).unwrap_or(true);
                }

                if terminate_oldest {
                    // Find oldest stream
                    user_clients.sort_by_key(|k| k.2);
                    if let Some((ch_id, cl_id, _)) = user_clients.first() {
                        tracing::info!("Terminating oldest stream {} for user {} to stay within limit {}", cl_id, user.username, limit);
                        let mut active = state.active_streams.write().await;
                        if let Some(stats) = active.get_mut(ch_id) {
                            stats.clients.retain(|c| c.client_id != *cl_id);
                            // The actual connection will drop when the stream finishes reading the next chunk or if we had a way to signal it.
                            // In this simple proxy, we don't have a direct handle to the task, 
                            // but when the client is removed from the stats, we could potentially drop the body stream.
                            // However, the Body::from_stream is already running.
                        }
                    }
                } else {
                    tracing::warn!("Blocking new stream for user {} (limit {} reached)", user.username, limit);
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
    }

    // Determine if identifier is a UUID (channel) or Hash (stream)
    let parsed_uuid = Uuid::parse_str(&channel_id).ok();

    let channel_streams = if let Some(uuid) = parsed_uuid {
        // Fetch the channel gracefully from Postgres
        let channel_opt = channel::Entity::find()
            .filter(channel::Column::Uuid.eq(uuid))
            .one(&state.db)
            .await
            .map_err(|e| {
                println!("DB Error fetching channel: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(_channel) = channel_opt {
            let parsed_id = _channel.id;

            channel_stream::Entity::find()
                .filter(channel_stream::Column::ChannelId.eq(parsed_id))
                .order_by_asc(channel_stream::Column::Order)
                .find_also_related(stream::Entity)
                .all(&state.db)
                .await
                .map_err(|e| {
                    println!("DB Error fetching channel streams: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        } else {
            get_stream_fallback(&channel_id, &state.db).await?
        }
    } else {
        get_stream_fallback(&channel_id, &state.db).await?
    };

    if channel_streams.is_empty() {
        println!("No streams found for identifier {}", channel_id);
        return Err(StatusCode::NOT_FOUND);
    }

    let mut successful_resp = None;

    for (_cs, stream_opt) in &channel_streams {
        if let Some(stream) = stream_opt {
            if let Some(target_url) = &stream.url {
                println!(
                    "▶️ Proxying Stream Identifier: {} -> {}",
                    channel_id, target_url
                );

                // Get the User-Agent from settings or fallback
                let mut provider_request = state.http_client.get(target_url)
                    .timeout(std::time::Duration::from_secs(30));

                // Apply Default User Agent if configured
                let stream_settings = crate::settings::get_setting_by_key(&state.db, "stream_settings").await;
                let mut ua_log = "Default".to_string();
                if let Some(ss) = stream_settings {
                    if let Some(ua) = ss.get("default_user_agent").and_then(|v| v.as_str()) {
                        provider_request = provider_request.header(axum::http::header::USER_AGENT, ua);
                        ua_log = ua.to_string();
                    }
                }

                tracing::info!("📡 FETCH: {} (User-Agent: {})", target_url, ua_log);

                let resp = provider_request.send().await;

                match resp {
                    Ok(r) if r.status().is_success() => {
                        successful_resp = Some(r);
                        break;
                    }
                    Ok(r) => {
                        println!(
                            "⚠️ Stream {} returned status {}, trying next",
                            target_url,
                            r.status()
                        );
                    }
                    Err(e) => {
                        println!("⚠️ Stream {} failed: {}, trying next", target_url, e);
                    }
                }
            }
        }
    }

    let resp = successful_resp.ok_or(StatusCode::BAD_GATEWAY)?;

    // 5. Zero-Copy Byte Streaming
    let client_id = Uuid::new_v4().to_string();
    let bytes_counter;
    {
        let mut active = state.active_streams.write().await;
        let stats = active
            .entry(channel_id.clone())
            .or_insert_with(|| ChannelStats {
                channel_id: channel_id.clone(),
                stream_id: channel_streams[0].0.stream_id.to_string(),
                stream_profile: "1".to_string(),
                provider_user_agent: ua_log.clone(),
                uptime: 0,
                total_bytes: Arc::new(AtomicU64::new(0)),
                clients: Vec::new(),
                start_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        stats.clients.push(ClientStat {
            client_id: client_id.clone(),
            user_id: authenticated_user.as_ref().map(|u| u.id),
            ip: client_ip,
            user_agent: client_user_agent.clone(),
            connected_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        bytes_counter = stats.total_bytes.clone();
    }

    let state_clone = state.clone();
    let channel_id_clone = channel_id.clone();
    let client_id_clone = client_id.clone();

    let stream = resp.bytes_stream().map(move |result| {
        // Periodically check if this client is still in the active list (for termination)
        // This is a bit expensive but allows us to terminate streams remotely.
        // We'll check every chunk, or maybe we should only check if we had a "dirty" flag.
        // For now, let's just use the guard to increment bytes.
        if let Ok(bytes) = &result {
            bytes_counter.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        }
        result.map_err(std::io::Error::other)
    });

    // Wrap the stream in a way that checks for termination
    let state_for_stream = state.clone();
    let channel_id_for_stream = channel_id.clone();
    let client_id_for_stream = client_id.clone();

    let monitored_stream = futures_util::stream::unfold(
        (stream, state_for_stream, channel_id_for_stream, client_id_for_stream),
        move |(mut s, st, ch, cl)| async move {
            // Check if client is still active
            {
                let active = st.active_streams.read().await;
                if let Some(stats) = active.get(&ch) {
                    if !stats.clients.iter().any(|c| c.client_id == cl) {
                        tracing::info!("Stopping stream {} for {} because it was removed from active list", cl, ch);
                        return None;
                    }
                } else {
                    return None;
                }
            }

            match s.next().await {
                Some(res) => Some((res, (s, st, ch, cl))),
                None => None,
            }
        },
    );

    let guard = Arc::new(ClientDropGuard {
        channel_id: channel_id_clone,
        client_id: client_id_clone,
        state: state_clone,
    });

    // Use a wrapper body that holds the guard
    let final_stream = monitored_stream.map(move |res| {
        let _g = &guard;
        res
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t")
        .body(Body::from_stream(final_stream))
        .unwrap())
}
