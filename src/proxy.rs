use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt; 
use sea_orm::{EntityTrait, QueryFilter, QueryOrder, ColumnTrait};
use std::sync::Arc;
use crate::{AppState, entities::{channel, channel_stream, stream}};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::auth::Claims;

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
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let parsed_id = channel_id.parse::<i64>().map_err(|_| StatusCode::BAD_REQUEST)?;

    // 2. Fetch the channel gracefully from Postgres
    let _channel = channel::Entity::find_by_id(parsed_id)
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 3. Determine Upstream URL
    let channel_streams = channel_stream::Entity::find()
        .filter(channel_stream::Column::ChannelId.eq(parsed_id))
        .order_by_asc(channel_stream::Column::Order)
        .all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut target_url = None;
    for cs in channel_streams {
        let stream_opt = stream::Entity::find_by_id(cs.stream_id)
            .one(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(s) = stream_opt {
            if let Some(url) = s.url {
                target_url = Some(url);
                break;
            }
        }
    }

    let target_url = target_url.ok_or(StatusCode::NOT_FOUND)?;

    println!("▶️ Proxying Stream Channel: {} -> {}", parsed_id, target_url);

    // 4. Request the Upstream bytes using our native Reqwest Client with timeouts
    let resp = state.http_client
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
    let stream = resp.bytes_stream().map(|result| {
        result.map_err(std::io::Error::other)
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t")
        .body(Body::from_stream(stream))
        .unwrap())
}