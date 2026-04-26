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

pub async fn handle_proxy(
    Path(channel_uuid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, StatusCode> {
    // We look up the channel using the UUID instead of token parsing

    let parsed_uuid = Uuid::parse_str(&channel_uuid).map_err(|_| StatusCode::BAD_REQUEST)?;

    // 2. Fetch the channel gracefully from Postgres
    let _channel = channel::Entity::find()
        .filter(channel::Column::Uuid.eq(parsed_uuid))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let parsed_id = _channel.id;

    // 3. Determine Upstream URL
    // Link with the ChannelStream entity mapped to `dispatcharr_channels_stream`
    // to pick the highest priority / active stream for the requested channel.

    let channel_streams = channel_stream::Entity::find()
        .filter(channel_stream::Column::ChannelId.eq(parsed_id))
        .order_by_asc(channel_stream::Column::Order)
        .find_also_related(stream::Entity)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if channel_streams.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut successful_resp = None;

    for (_cs, stream_opt) in channel_streams {
        if let Some(stream) = stream_opt {
            if let Some(target_url) = &stream.url {
                println!(
                    "▶️ Proxying Stream Channel: {} -> {}",
                    parsed_id, target_url
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
    // Stream the raw bytes directly to Axum to avoid consuming memory
    let stream = resp
        .bytes_stream()
        .map(|result| result.map_err(std::io::Error::other));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t")
        .body(Body::from_stream(stream))
        .unwrap())
}
