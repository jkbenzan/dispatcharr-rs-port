use crate::{
    entities::{channel, channel_stream, stream},
    AppState,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use axum::extract::ConnectInfo;
use std::net::SocketAddr;

#[derive(Serialize, Clone, Debug)]
pub struct ClientStat {
    pub client_id: String,
    pub ip: String,
    pub connected_at: u64,
}

#[derive(Clone, Debug)]
pub struct ChannelStats {
    pub channel_id: String,
    pub stream_id: String,
    pub stream_profile: String,
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
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let mut channels = Vec::new();
    for (_, stat) in active.iter() {
        channels.push(serde_json::json!({
            "channel_id": stat.channel_id,
            "stream_id": stat.stream_id,
            "stream_profile": stat.stream_profile,
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
) -> Result<Response<Body>, StatusCode> {
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
            // It parsed as a UUID, but no channel was found.
            // MD5 hashes can falsely parse as UUIDs, so fallback to Stream lookup.
            get_stream_fallback(&channel_id, &state.db).await?
        }
    } else {
        // It's not a UUID, so it must be a stream hash
        get_stream_fallback(&channel_id, &state.db).await?
    };

    if channel_streams.is_empty() {
        println!("No streams found for identifier {}", channel_id);
        return Err(StatusCode::NOT_FOUND);
    }

    let mut successful_resp = None;

    for (_cs, stream_opt) in channel_streams {
        if let Some(stream) = stream_opt {
            if let Some(target_url) = &stream.url {
                println!(
                    "▶️ Proxying Stream Identifier: {} -> {}",
                    channel_id, target_url
                );

                let resp = state
                    .http_client
                    .get(target_url)
                    .timeout(std::time::Duration::from_secs(15))
                    .send()
                    .await;

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
        let stats = active.entry(channel_id.clone()).or_insert_with(|| ChannelStats {
            channel_id: channel_id.clone(),
            stream_id: channel_streams[0].0.stream_id.to_string(),
            stream_profile: "1".to_string(),
            uptime: 0,
            total_bytes: Arc::new(AtomicU64::new(0)),
            clients: Vec::new(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
        stats.clients.push(ClientStat {
            client_id: client_id.clone(),
            ip: "127.0.0.1".to_string(), // In production, we'd extract IP from request ConnectInfo
            connected_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        });
        bytes_counter = stats.total_bytes.clone();
    }

    let guard = Arc::new(ClientDropGuard {
        channel_id,
        client_id,
        state,
    });

    let stream = resp.bytes_stream().map(move |result| {
        let _guard = guard.clone();
        if let Ok(bytes) = &result {
            bytes_counter.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        }
        result.map_err(std::io::Error::other)
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t")
        .body(Body::from_stream(stream))
        .unwrap())
}
