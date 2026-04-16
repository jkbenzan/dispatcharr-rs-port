use axum::{extract::State, http::StatusCode, Json};
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

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh: String,
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
        "access": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-xIs7V95v-9mE", 
        "refresh": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-xIs7V95v-9mE",
        "user": {
            "id": 1,
            "username": "admin",
            "is_superuser": true
        }
    }))
}

pub async fn refresh_token(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<Value>, StatusCode> {
    // 1. In a production environment, you would verify the refresh token cryptographically
    // (e.g., using a library like `jsonwebtoken`) and ensure it hasn't expired.
    if payload.refresh.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 2. You would then typically look up the user associated with this refresh token
    // in the database using `state.db` to ensure the user is still active.

    // 3. Finally, you'd generate and return a new access token.
    Ok(Json(json!({
        "access": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.refreshed-access-token"
    })))
}

pub async fn get_core_settings() -> Json<Value> {
    Json(json!({
        "app_name": "Dispatcharr",
        "registration_enabled": false,
        "allow_public_m3u": true,
        "proxy_enabled": true,
        "refresh_interval": 3600,
        "channel_profiles": [],
        "stream_profiles": [],
        "user_agents": [],
        "notification_types": []
    }))
}

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

pub async fn get_channel_groups() -> Json<Value> { Json(json!([])) }
pub async fn get_profiles() -> Json<Value> { Json(json!([])) }
pub async fn get_m3u_accounts() -> Json<Value> { Json(json!([])) }
pub async fn get_epg_sources() -> Json<Value> { Json(json!([])) }
pub async fn get_notifications() -> Json<Value> { Json(json!([])) }
pub async fn get_ids_stub() -> Json<Value> { Json(json!([])) }

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}