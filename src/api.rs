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
        "email": "admin@example.com",
        "is_superuser": true,
        "is_active": true,
        "is_staff": true,
        "permissions": ["*"],
        "profile": {
            "theme": "dark",
            "language": "en"
        }
    }))
}

pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "rust_access_token",
        "refresh": "rust_refresh_token",
        "user": { "username": "admin", "is_superuser": true }
    }))
}

pub async fn get_core_settings() -> Json<Value> {
    Json(json!({
        "app_name": "Dispatcharr",
        "registration_enabled": false,
        "allow_public_m3u": true,
        "proxy_enabled": true,
        "refresh_interval": 3600
    }))
}

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

// Seeding with one dummy item helps React components render instead of hanging
pub async fn get_channel_groups() -> Json<Value> { Json(json!([])) }
pub async fn get_profiles() -> Json<Value> { Json(json!([])) }

pub async fn get_m3u_accounts() -> Json<Value> { 
    Json(json!([{
        "id": 1,
        "name": "Dummy Account",
        "url": "http://example.com",
        "enabled": true
    }])) 
}

pub async fn get_epg_sources() -> Json<Value> { Json(json!([])) }
pub async fn get_notifications() -> Json<Value> { Json(json!([])) }
pub async fn get_ids_stub() -> Json<Value> { Json(json!([])) }

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}