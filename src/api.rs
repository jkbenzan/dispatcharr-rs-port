use axum::{extract::State, Json};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;

// --- Data Models ---

#[derive(Serialize, Deserialize)]
pub struct Channel {
    pub id: i32,
    pub name: String,
    pub stream_url: Option<String>,
    pub logo: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Group {
    pub id: i32,
    pub name: String,
}

// --- Handlers ---

/// Returns the system version and status. 
/// The frontend usually checks this immediately upon loading.
pub async fn get_system_status() -> Json<Value> {
    Json(json!({
        "version": "0.22.1",
        "status": "ok",
        "database": "connected",
        "engine": "rust-rs"
    }))
}

/// Placeholder for the channels list.
/// Later, you will update this to query the Postgres DB using SeaORM.
pub async fn get_channels(State(_state): State<Arc<AppState>>) -> Json<Vec<Channel>> {
    // For now, returning a static list to ensure the UI populates
    Json(vec![
        Channel { 
            id: 1, 
            name: "Sample Channel".into(), 
            stream_url: Some("http://localhost:8080/play/test/1".into()),
            logo: None 
        }
    ])
}

/// Placeholder for channel groups.
pub async fn get_groups(State(_state): State<Arc<AppState>>) -> Json<Vec<Group>> {
    Json(vec![
        Group { id: 1, name: "General".into() },
        Group { id: 2, name: "Sports".into() }
    ])
}

/// Optional: Configuration stub
/// Some versions of the UI call this to check for Auth or Theme settings.
pub async fn get_config() -> Json<Value> {
    Json(json!({
        "auth_enabled": false,
        "theme": "dark",
        "base_url": "/"
    }))
}