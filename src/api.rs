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
        "is_superuser": true
    }))
}

pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "rust_access_token",
        "refresh": "rust_refresh_token"
    }))
}

// --- NEW STUBS TO FIX 404s ---

pub async fn get_core_settings() -> Json<Value> {
    Json(json!({
        "app_name": "Dispatcharr",
        "registration_enabled": false
    }))
}

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false" }))
}

pub async fn get_channel_groups() -> Json<Value> {
    Json(json!([])) // Empty list for now
}

pub async fn get_profiles() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_m3u_accounts() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_epg_sources() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_notifications() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}