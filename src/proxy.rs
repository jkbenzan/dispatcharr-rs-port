use axum::{body::Body, extract::{Path, State}, response::{Response, IntoResponse}};
use std::sync::Arc;
use crate::AppState;
use futures_util::StreamExt;

pub async fn handle_proxy(
    Path((_token, channel_id)): Path<(String, i32)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let sources = vec![
        format!("http://example-provider.com/stream/{}.ts", channel_id),
    ];

    for url in sources {
        match state.http_client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let stream = resp.bytes_stream().map(|result| result.map_err(|e| e.to_string()));
                return Response::builder()
                    .header("Content-Type", "video/mp2t")
                    .body(Body::from_stream(stream))
                    .unwrap();
            }
            _ => continue,
        }
    }

    Response::builder().status(503).body(Body::from("All sources failed")).unwrap()
}
