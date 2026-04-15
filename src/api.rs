use axum::{extract::State, Json};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;

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

pub async fn get_system_status() -> Json<Value> {
    Json(json!({
        "version": "0.22.1",
        "status": "ok",
        "engine": "rust-rs"
    }))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({
        "auth_enabled": false,
        "theme": "dark",
        "base_url": "/"
    }))
}

pub async fn get_channels(State(_state): State<Arc<AppState>>) -> Json<Vec<Channel>> {
    Json(vec![
        Channel { 
            id: 1, 
            name: "Sample Channel".into(), 
            stream_url: Some("/play/test/1".into()),
            logo: None 
        }
    ])
}

pub async fn get_groups(State(_state): State<Arc<AppState>>) -> Json<Vec<Group>> {
    Json(vec![
        Group { id: 1, name: "General".into() }
    ])
}