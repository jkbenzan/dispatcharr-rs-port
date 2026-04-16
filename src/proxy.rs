use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt; 
use std::sync::Arc;
use crate::AppState;

pub async fn handle_proxy(
    Path((_token, channel_id)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Response<Body>, StatusCode> {
    // Note: In the future, query your DB for the actual source URL using channel_id
    let target_url = format!("http://example.com/stream/{}", channel_id);

    let resp = state.http_client
        .get(target_url)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Convert the reqwest bytes_stream into an Axum-compatible Body stream
    let stream = resp.bytes_stream().map(|result| {
        result.map_err(std::io::Error::other)
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "video/mp2t")
        .body(Body::from_stream(stream))
        .unwrap())
}