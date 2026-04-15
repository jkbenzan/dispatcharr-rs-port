use axum::{extract::State, Json};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::AppState;

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
    // Valid Base64 JWT-like strings to avoid DOMException
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

// Wraps empty lists in a "results" object to satisfy common frontend patterns
pub async fn get_results_stub() -> Json<Value> {
    Json(json!({
        "count": 0,
        "next": null,
        "previous": null,
        "results": []
    }))
}

pub async fn get_env_settings() -> Json<Value> {
    Json(json!({ "DEBUG": "false", "ENV": "production" }))
}

pub async fn get_config() -> Json<Value> {
    Json(json!({ "auth_enabled": false, "theme": "dark", "base_url": "/" }))
}