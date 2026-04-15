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

pub async fn get_core_version() -> Json<Value> {
    Json(json!({
        "version": "0.22.1",
        "name": "Dispatcharr",
        "description": "Rust Backend"
    }))
}

pub async fn check_superuser() -> Json<Value> {
    Json(json!({ "initialized": true }))
}

pub async fn get_current_user() -> Json<Value> {
    Json(json!({
        "id": 1,
        "username": "admin",
        "email": "admin@example.com",
        "is_superuser": true,
        "first_name": "Admin",
        "last_name": "User"
    }))
}

// Fixed: This returns the standard JWT-style response the UI expects
pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "rust_access_token_placeholder",
        "refresh": "rust_refresh_token_placeholder",
        "user": {
            "username": "admin",
            "is_superuser": true
        }
    }))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
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
    Json(vec![Group { id: 1, name: "General".into() }])
}