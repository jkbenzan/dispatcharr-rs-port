use axum::Json;
use serde_json::{json, Value};

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
    // Adding channel_profiles here as the console specifically asked for it
    Json(json!({
        "app_name": "Dispatcharr",
        "proxy_enabled": true,
        "channel_profiles": [],
        "stream_profiles": [],
        "user_agents": [],
        "notification_types": []
    }))
}

// Fixed: The frontend needs a flat array [] so it can call .filter() or .reduce()
pub async fn get_flat_list() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}

pub async fn logout_stub() -> Json<Value> {
    Json(json!({ "detail": "Successfully logged out." }))
}