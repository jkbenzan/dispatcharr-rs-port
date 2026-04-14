use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::AppState;

#[derive(Serialize)]
pub struct Channel {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize)]
pub struct Group {
    pub id: i32,
    pub name: String,
}

// Existing channels route
pub async fn get_channels(State(_state): State<Arc<AppState>>) -> Json<Vec<Channel>> {
    Json(vec![
        Channel { id: 1, name: "Sample Channel".into() }
    ])
}

// ADD THIS: Placeholder for the groups route
pub async fn get_groups(State(_state): State<Arc<AppState>>) -> Json<Vec<Group>> {
    Json(vec![
        Group { id: 1, name: "General".into() }
    ])
}