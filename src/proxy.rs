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
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
pub enum BroadcasterStatus {
    Connecting,
    Streaming,
    Buffering,
    Failover,
    Stopping,
}

pub struct Broadcaster {
    pub channel_id: String,
    pub tx: broadcast::Sender<bytes::Bytes>,
    pub subscriber_count: Arc<AtomicUsize>,
    pub ring_buffer: Arc<RwLock<VecDeque<bytes::Bytes>>>,
    pub status: Arc<RwLock<BroadcasterStatus>>,
    pub provider_ua: Arc<RwLock<String>>,
    pub stream_id: Arc<RwLock<Option<String>>>,
    pub start_time: u64,
    pub total_bytes: Arc<AtomicU64>,
    pub pumper_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ClientStat {
    pub client_id: String,
    pub user_id: Option<i64>,
    pub ip_address: String, // Renamed to match frontend
    pub user_agent: String,
    pub connected_at: u64,
    pub connection_duration: f64, // Added for frontend
    pub connected_since: f64,    // Added for frontend
}

#[derive(Serialize, Clone, Debug)] // Added Serialize
pub struct ChannelStats {
    pub channel_id: String,
    pub stream_id: String,
    pub stream_profile: String,
    pub provider_user_agent: String,
    pub uptime: u64,
    pub total_bytes: Arc<AtomicU64>,
    pub clients: Vec<ClientStat>,
    pub start_time: u64,
    pub m3u_profile: Option<serde_json::Value>, // Added for frontend
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
            
            // Decrement subscriber count
            if let Some(b) = state.broadcasters.get(&channel_id) {
                b.subscriber_count.fetch_sub(1, Ordering::SeqCst);
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
    identifier: &str,
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<(channel_stream::Model, Option<stream::Model>)>, StatusCode> {
    tracing::debug!("Checking fallback identifier: {}", identifier);
    
    // 1. Try numeric ID
    if let Ok(id) = identifier.parse::<i64>() {
        tracing::debug!("Searching for streams with channel_id: {}", id);
        let streams = channel_stream::Entity::find()
            .filter(channel_stream::Column::ChannelId.eq(id))
            .order_by_asc(channel_stream::Column::Order)
            .find_also_related(stream::Entity)
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("get_stream_fallback DB Error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        if !streams.is_empty() {
            tracing::debug!("Found {} streams for channel_id {}", streams.len(), id);
            return Ok(streams);
        }
    }

    // 2. Try UUID as string match on Channel
    tracing::debug!("Searching for channel with UUID string match: {}", identifier);
    let channel_opt = channel::Entity::find()
        .filter(channel::Column::Uuid.eq(identifier))
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("get_stream_fallback DB Error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if let Some(_channel) = channel_opt {
        tracing::debug!("Found channel by UUID string: {}", _channel.name);
        let streams = channel_stream::Entity::find()
            .filter(channel_stream::Column::ChannelId.eq(_channel.id))
            .order_by_asc(channel_stream::Column::Order)
            .find_also_related(stream::Entity)
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("get_stream_fallback DB Error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        if !streams.is_empty() {
            return Ok(streams);
        }
    }

    // 3. Try Stream Hash
    tracing::debug!("Searching for stream with hash: {}", identifier);
    let stream_opt = stream::Entity::find()
        .filter(stream::Column::StreamHash.eq(identifier))
        .one(db)
        .await
        .map_err(|e| {
            tracing::error!("get_stream_fallback DB Error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if let Some(_stream) = stream_opt {
        tracing::debug!("Found stream by hash: {}", _stream.id);
        return Ok(vec![(
            channel_stream::Model {
                id: 0,
                channel_id: 0,
                stream_id: _stream.id,
                order: 0,
            },
            Some(_stream),
        )]);
    }

    tracing::warn!("No match found for fallback identifier {}", identifier);
    Err(StatusCode::NOT_FOUND)
}

pub async fn handle_ts_status(State(state): State<Arc<AppState>>) -> axum::Json<serde_json::Value> {
    let active = state.active_streams.read().await;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut channels = Vec::new();
    for (_, stat) in active.iter() {
        let mut clients = Vec::new();
        for client in &stat.clients {
            let duration = now.saturating_sub(client.connected_at) as f64;
            clients.push(serde_json::json!({
                "client_id": client.client_id,
                "user_id": client.user_id,
                "ip_address": client.ip_address,
                "user_agent": client.user_agent,
                "connected_at": client.connected_at,
                "connection_duration": duration,
                "connected_since": duration,
            }));
        }

        channels.push(serde_json::json!({
            "channel_id": stat.channel_id,
            "stream_id": stat.stream_id,
            "stream_profile": stat.stream_profile,
            "provider_user_agent": stat.provider_user_agent,
            "uptime": now.saturating_sub(stat.start_time),
            "total_bytes": stat.total_bytes.load(Ordering::Relaxed),
            "clients": clients,
            "m3u_profile": stat.m3u_profile,
        }));
    }

    axum::Json(serde_json::json!({
        "channels": channels
    }))
}

pub async fn handle_vod_stats() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!([]))
}

pub async fn broadcaster_pumper(
    channel_id: String,
    state: Arc<AppState>,
    channel_streams: Vec<(channel_stream::Model, Option<stream::Model>)>,
    broadcaster: Arc<Broadcaster>,
) {
    let settings = crate::settings::get_proxy_settings(&state.db).await;
    let mut retry_count = 0;
    let mut stream_index = 0;
    
    loop {
        if stream_index >= channel_streams.len() {
            tracing::error!("All streams failed for channel {}", channel_id);
            *broadcaster.status.write().await = BroadcasterStatus::Failover;
            break;
        }

        let (_cs, stream_opt) = &channel_streams[stream_index];
        let stream = match stream_opt {
            Some(s) => s,
            None => {
                stream_index += 1;
                continue;
            }
        };

        let target_url = match &stream.url {
            Some(url) => url,
            None => {
                stream_index += 1;
                continue;
            }
        };

        *broadcaster.stream_id.write().await = Some(stream.id.to_string());
        *broadcaster.status.write().await = BroadcasterStatus::Connecting;

        let mut provider_request = state.http_client.get(target_url);
        let stream_settings = crate::settings::get_setting_by_key(&state.db, "stream_settings").await;
        if let Some(ss) = stream_settings {
            if let Some(ua) = ss.get("default_user_agent").and_then(|v| v.as_str()) {
                provider_request = provider_request.header(axum::http::header::USER_AGENT, ua);
                *broadcaster.provider_ua.write().await = ua.to_string();
            }
        }

        tracing::info!("📡 Broadcaster connecting to: {}", target_url);

        let connect_result = tokio::time::timeout(
            std::time::Duration::from_secs(settings.buffering_timeout),
            provider_request.send()
        ).await;

        match connect_result {
            Ok(Ok(resp)) if resp.status().is_success() => {
                *broadcaster.status.write().await = BroadcasterStatus::Streaming;
                let mut bytes_stream = resp.bytes_stream();
                let mut no_subscribers_since: Option<tokio::time::Instant> = None;

                loop {
                    let subs = broadcaster.subscriber_count.load(Ordering::Relaxed);
                    if subs == 0 {
                        if no_subscribers_since.is_none() {
                            no_subscribers_since = Some(tokio::time::Instant::now());
                            *broadcaster.status.write().await = BroadcasterStatus::Stopping;
                        } else if no_subscribers_since.unwrap().elapsed().as_secs() > settings.channel_shutdown_delay {
                            tracing::info!("Channel {} shutdown delay reached, terminating broadcaster", channel_id);
                            state.broadcasters.remove(&channel_id);
                            return;
                        }
                    } else {
                        if no_subscribers_since.is_some() {
                            no_subscribers_since = None;
                            *broadcaster.status.write().await = BroadcasterStatus::Streaming;
                        }
                    }

                    let chunk_result = tokio::time::timeout(
                        std::time::Duration::from_secs(settings.buffering_timeout),
                        bytes_stream.next()
                    ).await;

                    match chunk_result {
                        Ok(Some(Ok(bytes))) => {
                            broadcaster.total_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                            
                            {
                                let mut rb = broadcaster.ring_buffer.write().await;
                                rb.push_back(bytes.clone());
                                // Limit ring buffer to roughly chunk_size bytes
                                let mut current_size: usize = rb.iter().map(|b| b.len()).sum();
                                while current_size > settings.chunk_size && !rb.is_empty() {
                                    if let Some(front) = rb.pop_front() {
                                        current_size -= front.len();
                                    }
                                }
                            }

                            let _ = broadcaster.tx.send(bytes);
                        }
                        Ok(Some(Err(e))) => {
                            tracing::warn!("Stream read error for {}: {}", target_url, e);
                            break;
                        }
                        Ok(None) => {
                            tracing::info!("Stream {} ended naturally", target_url);
                            break;
                        }
                        Err(_) => {
                            tracing::warn!("Stream {} buffering timeout!", target_url);
                            *broadcaster.status.write().await = BroadcasterStatus::Buffering;
                            break;
                        }
                    }
                }
            }
            _ => {
                tracing::warn!("Failed to connect to {}, triggering failover", target_url);
            }
        }

        retry_count += 1;
        if retry_count >= settings.max_retries {
            retry_count = 0;
            stream_index += 1;
        }
    }
    
    state.broadcasters.remove(&channel_id);
}

pub async fn get_or_create_broadcaster(
    channel_id: String,
    state: Arc<AppState>,
    channel_streams: Vec<(channel_stream::Model, Option<stream::Model>)>,
) -> Arc<Broadcaster> {
    if let Some(b) = state.broadcasters.get(&channel_id) {
        return b.clone();
    }

    let (tx, _) = tokio::sync::broadcast::channel(1024);
    let broadcaster = Arc::new(Broadcaster {
        channel_id: channel_id.clone(),
        tx,
        subscriber_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        ring_buffer: Arc::new(tokio::sync::RwLock::new(std::collections::VecDeque::new())),
        status: Arc::new(tokio::sync::RwLock::new(BroadcasterStatus::Connecting)),
        provider_ua: Arc::new(tokio::sync::RwLock::new(String::new())),
        stream_id: Arc::new(tokio::sync::RwLock::new(None)),
        start_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        total_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        pumper_handle: Arc::new(tokio::sync::RwLock::new(None)),
    });

    state.broadcasters.insert(channel_id.clone(), broadcaster.clone());

    let b_clone = broadcaster.clone();
    let handle = tokio::spawn(broadcaster_pumper(channel_id, state, channel_streams, b_clone));
    
    *broadcaster.pumper_handle.write().await = Some(handle);

    broadcaster
}

pub async fn handle_proxy(
    Path(channel_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProxyQuery>,
    headers: axum::http::HeaderMap,
) -> Result<Response<Body>, StatusCode> {
    let addr: std::net::SocketAddr = "127.0.0.1:12345".parse().unwrap();
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
        use jsonwebtoken::{decode, DecodingKey, Validation};
        use crate::auth::{Claims, JWT_SECRET};
        
        if let Ok(token_data) = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        ) {
            if let Ok(Some(user)) = user::Entity::find_by_id(token_data.claims.user_id).one(&state.db).await {
                authenticated_user = Some(user);
            }
        }
    }

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
    let mut current_channel_model = None;

    let channel_streams = if let Some(uuid) = parsed_uuid {
        let channel_opt = channel::Entity::find()
            .filter(channel::Column::Uuid.eq(uuid))
            .one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("DB Error fetching channel: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(_channel) = channel_opt {
            current_channel_model = Some(_channel.clone());
            let parsed_id = _channel.id;

            let channel_streams = channel_stream::Entity::find()
                .filter(channel_stream::Column::ChannelId.eq(parsed_id))
                .order_by_asc(channel_stream::Column::Order)
                .find_also_related(stream::Entity)
                .all(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("DB Error fetching channel streams: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            
            tracing::info!("Found {} streams for channel {}", channel_streams.len(), parsed_id);
            channel_streams
        } else {
            get_stream_fallback(&channel_id, &state.db).await?
        }
    } else {
        get_stream_fallback(&channel_id, &state.db).await?
    };

    if channel_streams.is_empty() {
        tracing::warn!("No streams found for identifier {}", channel_id);
        return Err(StatusCode::NOT_FOUND);
    }

    let broadcaster = get_or_create_broadcaster(channel_id.clone(), state.clone(), channel_streams.clone()).await;
    
    // 5. Zero-Copy Byte Streaming
    let client_id = Uuid::new_v4().to_string();
    
    // Fetch M3U Account info for the profile name
    let mut m3u_profile_data = None;
    if let Some(acc_id) = channel_streams[0].1.as_ref().and_then(|s| s.m3u_account_id) {
        if let Ok(Some(acc)) = crate::entities::m3u_account::Entity::find_by_id(acc_id).one(&state.db).await {
            m3u_profile_data = Some(serde_json::json!({
                "account_name": acc.name,
                "name": acc.name, // Renamed from profile_name to match frontend
            }));
        }
    }

    let provider_user_agent = {
        let ua = broadcaster.provider_ua.read().await;
        ua.clone()
    };

    {
        let mut active = state.active_streams.write().await;
        let stats = active
            .entry(channel_id.clone())
            .or_insert_with(|| {
                let profile_id = current_channel_model
                    .as_ref()
                    .and_then(|ch| ch.stream_profile_id)
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "Proxy".to_string());

                ChannelStats {
                    channel_id: channel_id.clone(),
                    stream_id: channel_streams[0].0.stream_id.to_string(),
                    stream_profile: profile_id,
                    provider_user_agent: provider_user_agent,
                    uptime: 0,
                    total_bytes: broadcaster.total_bytes.clone(),
                    clients: Vec::new(),
                    start_time: broadcaster.start_time,
                    m3u_profile: m3u_profile_data,
                }
            });
            
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        stats.clients.push(ClientStat {
            client_id: client_id.clone(),
            user_id: authenticated_user.as_ref().map(|u| u.id),
            ip_address: client_ip.clone(),
            user_agent: client_user_agent.clone(),
            connected_at: now,
            connection_duration: 0.0,
            connected_since: 0.0,
        });
    }

    broadcaster.subscriber_count.fetch_add(1, Ordering::SeqCst);

    // Record Event: Stream Started
    let channel_name = current_channel_model.as_ref().map(|c| c.name.clone());
    let _ = crate::events::record_event(
        &state.db,
        "channel_start",
        channel_name,
        serde_json::json!({
            "channel_id": current_channel_model.as_ref().map(|c| c.id),
            "ip_address": client_ip.clone(),
            "user_agent": client_user_agent.clone(),
            "user": authenticated_user.as_ref().map(|u| u.username.clone()).unwrap_or("Anonymous".to_string())
        })
    ).await;

    let guard = Arc::new(ClientDropGuard {
        channel_id: channel_id.clone(),
        client_id: client_id.clone(),
        state: state.clone(),
    });

    let rx = broadcaster.tx.subscribe();
    let ring_data = {
        let rb = broadcaster.ring_buffer.read().await;
        rb.clone()
    };
    
    let state_for_stream = state.clone();
    let channel_id_for_stream = channel_id.clone();
    let client_id_for_stream = client_id.clone();
    
    let stream = futures_util::stream::unfold(
        (ring_data.into_iter(), rx, state_for_stream, channel_id_for_stream, client_id_for_stream),
        move |(mut ring_iter, mut rx, st, ch, cl)| async move {
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

            if let Some(bytes) = ring_iter.next() {
                return Some((Ok::<_, std::io::Error>(bytes), (ring_iter, rx, st, ch, cl)));
            }
            
            loop {
                match rx.recv().await {
                    Ok(bytes) => return Some((Ok(bytes), (ring_iter, rx, st, ch, cl))),
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                }
            }
        }
    );

    let final_stream = stream.map(move |res| {
        let _g = &guard;
        res
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t")
        .body(Body::from_stream(final_stream))
        .unwrap())
}
