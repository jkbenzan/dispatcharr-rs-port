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
        "email": "admin@example.com",
        "is_superuser": true,
        "is_active": true,
        "is_staff": true,
        "permissions": ["*"],
        "profile": { "theme": "dark", "language": "en" }
    }))
}

pub async fn auth_placeholder() -> Json<Value> {
    Json(json!({ 
        "access": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6ImFkbWluIiwiaXNfc3VwZXJ1c2VyIjp0cnVlfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c", 
        "refresh": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VyX2lkIjoxLCJ1c2VybmFtZSI6ImFkbWluIiwiaXNfc3VwZXJ1c2VyIjp0cnVlfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        "user": { "id": 1, "username": "admin", "is_superuser": true, "is_staff": true }
    }))
}

pub async fn get_core_settings() -> Json<Value> {
    // Adding EVERY standard key. The 'length' crash happens when one of these arrays is missing.
    Json(json!({
        "app_name": "Dispatcharr",
        "proxy_enabled": true,
        "registration_enabled": false,
        "channel_profiles": [],
        "stream_profiles": [],
        "user_agents": [],
        "notification_types": [],
        "backend_url": "",
        "version": "0.22.1",
        "maintenance_mode": false
    }))
}

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

// Fixed: Frontend calls .filter() on this, so it MUST be an object with results
pub async fn get_notifications() -> Json<Value> {
    Json(json!({
        "count": 0,
        "next": null,
        "previous": null,
        "results": []
    }))
}

// Raw array for components calling .reduce()
pub async fn get_flat_list() -> Json<Value> {
    Json(json!([]))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}

pub async fn logout_stub() -> Json<Value> {
    Json(json!({ "detail": "Successfully logged out." }))
}