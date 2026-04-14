use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;

#[derive(Serialize)]
pub struct Channel {
    id: i32,
    name: String,
}

pub async fn get_channels(State(_state): State<Arc<AppState>>) -> Json<Vec<Channel>> {
    Json(vec![
        Channel { id: 1, name: "Sample Channel".into() }
    ])
}
