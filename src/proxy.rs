use crate::auth::Claims;
use crate::{entities::channel, AppState};
use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use jsonwebtoken::{decode, DecodingKey, Validation};
use sea_orm::EntityTrait;
use std::sync::Arc;

const STREAM_SECRET: &[u8] = b"dispatcharr_super_secret_temporary_key";

pub async fn handle_proxy(
    Path((token, channel_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, StatusCode> {
    // 1. Authenticate the Token
    // We decode the token to ensure the player making the GET request has an active session or an API key
    let _token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(STREAM_SECRET),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let parsed_id = channel_id
        .parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 2. Fetch the channel gracefully from Postgres
    let _channel = channel::Entity::find_by_id(parsed_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 3. Determine Upstream URL
    // TODO: Link with the ChannelStream entity mapped to `dispatcharr_channels_stream`
    // to pick the highest priority / active stream for the requested channel.
    let target_url = "http://example.com/test_stream.m3u8".to_string(); // Placeholder

    println!(
        "▶️ Proxying Stream Channel: {} -> {}",
        parsed_id, target_url
    );

    // 4. Request the Upstream bytes using our native Reqwest Client with timeouts
    let resp = state
        .http_client
        .get(&target_url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        // If it fails, failover logic would trigger here to grab the next priority URL
        return Err(StatusCode::NOT_FOUND);
    }

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
