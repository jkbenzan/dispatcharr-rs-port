use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt; // This was missing the crate in Cargo.toml
use std::sync::Arc;
use crate::AppState;

pub async fn handle_proxy(
    Path((_token, channel_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, StatusCode> {
    // 1. In a real scenario, you'd look up the actual M3U URL for this channel_id in Postgres
    // For now, we'll use a placeholder URL to test the stream logic
    let target_url = format!("http://example.com/stream/{}", channel_id);

    let resp = state.http_client
        .get(target_url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        return Err(StatusCode::NOT_FOUND);
    }

    // 2. Convert the reqwest response into a stream for Axum
    // bytes_stream() requires the "stream" feature in Cargo.toml
    let stream = resp.bytes_stream().map(|result| {
        result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t") // Standard for IPTV TS streams
        .body(Body::from_stream(stream))
        .unwrap())
}