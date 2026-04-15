use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;

pub async fn get_core_version() -> Json<Value> {
    Json(json!({ "version": "0.22.1", "name": "Dispatcharr" }))
}

pub async fn check_superuser() -> Json<Value> {
    Json(json!({ "initialized": true }))
}

pub async fn get_current_user() -> Json<Value> {
    Json(json!({
        "id": 1,
        "username": "admin",
        "is_superuser": true,
        "is_active": true,
        "is_staff": true,
        "permissions": ["*"],
        "profile": { "theme": "dark" }
    }))
}

pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-xIs7V95v-9mE", 
        "refresh": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.t-xIs7V95v-9mE"
    }))
}

pub async fn get_core_settings() -> Json<Value> {
    // The UI CRASHES if these are missing or undefined
    Json(json!({
        "app_name": "Dispatcharr",
        "proxy_enabled": true,
        "channel_profiles": [],
        "stream_profiles": [],
        "user_agents": [],
        "notification_types": []
    }))
}

// Fixed: The frontend console shows it wants to call .reduce() and .filter() 
// This requires a FLAT ARRAY [], not the {"results": []} wrapper.
pub async fn get_flat_list() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}